use super::characters::{AbilityKind, BuffKind, BuildKind, ProjKind, WeatherKind};
use super::config::*;
use super::fx::ParticleShape;
use super::math::{clamp, dist};
use super::sim::{find_free_x, Entity, EntityKind, Event, Freeze, Match, Player, Weather};

const TAU: f32 = std::f32::consts::TAU;
const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;

fn tint(side: usize) -> [u8; 3] {
    if side == 0 {
        [142, 230, 79]
    } else {
        [255, 143, 199]
    }
}

pub fn block_reason(m: &Match, p: &Player) -> Option<&'static str> {
    let def = p.spec().ability;
    if matches!(def.kind, AbilityKind::None) {
        return Some("none");
    }
    if m.phase != super::sim::Phase::Playing {
        return Some("phase");
    }
    if p.frozen {
        return Some("frozen");
    }
    if p.mana + 1e-6 < def.cost {
        return Some("mana");
    }
    if p.cooldown > 0.0 {
        return Some("cooldown");
    }

    let opp = &m.players[1 - p.side];
    match def.kind {
        AbilityKind::Shrink { range, .. } | AbilityKind::Drain { range, .. } => {
            if dist(p.x, p.y, opp.x, opp.y) > range {
                return Some("range");
            }
        }
        AbilityKind::Reverse { .. } => {
            if dist(p.x, p.y, m.ball.x, m.ball.y) > 300.0 {
                return Some("range");
            }
        }
        AbilityKind::SuperJump { .. } if !p.on_ground => return Some("air"),
        AbilityKind::Freeze { .. } if m.freeze.is_some() => return Some("active"),
        AbilityKind::Clone { .. }
            if m.entities.iter().any(|e| {
                matches!(e.kind, EntityKind::Clone { .. }) && e.owner == p.side && e.life > 0.0
            }) =>
        {
            return Some("active");
        }
        _ => {}
    }
    None
}

pub fn activate(m: &mut Match, side: usize) -> bool {
    let spec = m.players[side].spec();
    let def = spec.ability;
    if let Some(reason) = block_reason(m, &m.players[side]) {
        match reason {
            "mana" => {
                m.add_event(Event::Deny { side, reason });
                m.fx.popup(
                    "NOT ENOUGH MANA",
                    m.players[side].x,
                    m.players[side].y - m.players[side].h / 2.0 - 12.0,
                    [255, 154, 154],
                    0.9,
                    7.0,
                );
            }
            "range" | "air" | "active" => {
                m.add_event(Event::Deny { side, reason });
                let words = match reason {
                    "range" => "OUT OF RANGE",
                    "air" => "NEED GROUND",
                    _ => "ALREADY ACTIVE",
                };
                m.fx.popup(
                    words,
                    m.players[side].x,
                    m.players[side].y - m.players[side].h / 2.0 - 12.0,
                    [255, 210, 122],
                    0.9,
                    7.0,
                );
            }
            "none" => m.add_event(Event::Deny { side, reason }),
            _ => {}
        }
        return false;
    }

    {
        let p = &mut m.players[side];
        p.mana = (p.mana - def.cost).max(0.0);
        p.cooldown = def.cooldown;
    }
    m.stats.abilities[side] += 1;
    m.add_event(Event::Ability {
        side,
        name: def.name,
    });
    let (px, py, pw, p_ph, facing) = {
        let p = &m.players[side];
        (p.x, p.y, p.w, p.h, p.facing)
    };
    m.fx.popup(
        def.name.to_uppercase(),
        px,
        py - p_ph / 2.0 - 14.0,
        if side == 0 {
            [200, 255, 150]
        } else {
            [255, 200, 230]
        },
        0.95,
        7.0,
    );
    let colors = [tint(side), [255, 255, 255]];
    m.fx.burst(
        &mut m.rng,
        px,
        py,
        &colors,
        8,
        65.0,
        0.4,
        -30.0,
        2.0,
        -HALF_PI,
        TAU,
        ParticleShape::Circle,
    );

    let dir = if side == 0 { 1.0 } else { -1.0 };
    let opp = 1 - side;
    let own_dir = -dir;

    match def.kind {
        AbilityKind::None => {}
        AbilityKind::Projectile {
            kind,
            speed,
            impulse,
            pull,
        } => {
            let sx = px + facing * 10.0;
            let sy = py - p_ph * 0.28;
            let mut dx = m.ball.x - sx;
            let mut dy = m.ball.y - sy;
            if dx * facing < -24.0 {
                dx = facing * 50.0;
                dy = -6.0;
            }
            let len = dx.hypot(dy).max(1.0);
            let (r, gravity, life) = match kind {
                ProjKind::Bullet => (2.0, 0.0, 0.8),
                ProjKind::Fireball => (5.0, 0.0, 1.35),
                ProjKind::Glove => (8.0, 0.0, 0.55),
                ProjKind::Harpoon | ProjKind::Hook => (3.0, 0.0, 1.15),
                ProjKind::Bomb => (6.0, 540.0, 1.5),
                ProjKind::Arrow | ProjKind::Boomerang => (3.0, 0.0, 1.4),
            };
            let mut vx = dx / len * speed;
            let mut vy = dy / len * speed;
            if kind == ProjKind::Bomb {
                vy = -230.0 + dy.min(0.0) * 0.4;
            }
            let id = m.next_id();
            m.entities.push(Entity {
                id,
                owner: side,
                side,
                x: sx,
                y: sy,
                vx,
                vy,
                w: r * 2.0,
                h: r * 2.0,
                life,
                max_life: life,
                dead: false,
                kind: EntityKind::Projectile {
                    kind,
                    r,
                    gravity,
                    impulse,
                    pull,
                    age: 0.0,
                    returning: false,
                    trail: 0.0,
                },
            });
            let _ = (&mut vx, &mut vy);
        }
        AbilityKind::Pull { force, duration } => {
            let p = &mut m.players[side];
            p.pull_timer = duration;
            p.pull_force = force;
            m.fx.beam(
                px,
                py - 8.0,
                m.ball.x,
                m.ball.y,
                tint(side),
                duration * 0.35,
                2.0,
                false,
            );
        }
        AbilityKind::Remote { force, duration } => {
            let p = &mut m.players[side];
            p.remote_timer = duration;
            p.remote_force = force;
            m.fx.beam(
                px,
                py - p_ph * 0.4,
                m.ball.x,
                m.ball.y,
                [75, 229, 255],
                0.5,
                1.0,
                true,
            );
        }
        AbilityKind::Frost { radius, duration } => {
            m.ball.vx *= 0.45;
            m.ball.vy *= 0.45;
            m.ball.chill_timer = m.ball.chill_timer.max(duration);
            if dist(px, py, m.players[opp].x, m.players[opp].y) <= radius {
                let b = &mut m.players[opp].buffs;
                b.chill = b.chill.max(duration);
                m.refresh_player_size(opp);
            }
            m.fx.ring(px, py, radius, [189, 246, 255], 0.55, 3.0);
            let colors = [[217, 251, 255], [122, 217, 245], [255, 255, 255]];
            m.fx.burst(
                &mut m.rng,
                px,
                py,
                &colors,
                26,
                160.0,
                0.7,
                20.0,
                2.0,
                -HALF_PI,
                TAU,
                ParticleShape::Circle,
            );
        }
        AbilityKind::Shrink { duration, .. } => {
            let b = &mut m.players[opp].buffs;
            b.shrink = b.shrink.max(duration);
            m.refresh_player_size(opp);
            let (ox, oy, oh) = {
                let o = &m.players[opp];
                (o.x, o.y, o.h)
            };
            m.fx.beam(
                px,
                py - p_ph * 0.4,
                ox,
                oy - oh * 0.3,
                [230, 255, 154],
                0.35,
                3.0,
                false,
            );
            let colors = [[230, 255, 154], [255, 255, 255]];
            m.fx.burst(
                &mut m.rng,
                ox,
                oy,
                &colors,
                16,
                90.0,
                0.6,
                -40.0,
                2.0,
                -HALF_PI,
                TAU,
                ParticleShape::Circle,
            );
            m.fx.popup(
                "SHRUNK!",
                ox,
                oy - oh / 2.0 - 12.0,
                [230, 255, 154],
                1.0,
                7.0,
            );
        }
        AbilityKind::Drain { duration, .. } => {
            {
                let b = &mut m.players[opp].buffs;
                b.drain_slow = b.drain_slow.max(duration);
                let b = &mut m.players[side].buffs;
                b.drain_boost = b.drain_boost.max(duration);
                let p = &mut m.players[side];
                p.drain_timer = duration;
                p.drain_target = Some(opp);
            }
            m.refresh_player_size(opp);
            m.refresh_player_size(side);
            let (ox, oy, oh) = {
                let o = &m.players[opp];
                (o.x, o.y, o.h)
            };
            m.fx.beam(
                px,
                py - p_ph * 0.4,
                ox,
                oy - oh * 0.4,
                [255, 77, 109],
                duration,
                2.0,
                false,
            );
            m.fx.popup(
                "DRAINED!",
                ox,
                oy - oh / 2.0 - 12.0,
                [255, 143, 165],
                1.0,
                7.0,
            );
        }
        AbilityKind::Freeze { duration } => {
            m.freeze = Some(Freeze {
                side,
                timer: duration,
                total: duration,
            });
            m.ball.vx = 0.0;
            m.ball.vy = 0.0;
            let bx = m.ball.x;
            let by = m.ball.y;
            m.fx.ring(bx, by, 90.0, [255, 243, 200], 0.7, 3.0);
            let colors = [[255, 243, 200], [255, 255, 255], [199, 139, 58]];
            m.fx.burst(
                &mut m.rng,
                bx,
                by,
                &colors,
                22,
                120.0,
                0.9,
                0.0,
                2.0,
                -HALF_PI,
                TAU,
                ParticleShape::Circle,
            );
            m.shake = m.shake.max(4.0);
        }
        AbilityKind::Buff { kind, duration } => {
            {
                let b = &mut m.players[side].buffs;
                match kind {
                    BuffKind::Sprint => b.sprint = b.sprint.max(duration),
                    BuffKind::Rage => b.rage = b.rage.max(duration),
                    BuffKind::Pixel => b.pixel = b.pixel.max(duration),
                    BuffKind::Grow => b.grow = b.grow.max(duration),
                    BuffKind::Invisibility => b.invisible = b.invisible.max(duration),
                }
            }
            m.refresh_player_size(side);
            let max_r = if kind == BuffKind::Invisibility {
                45.0
            } else {
                60.0
            };
            let ring_color = if kind == BuffKind::Pixel {
                [255, 93, 241]
            } else {
                tint(side)
            };
            m.fx.ring(px, py, max_r, ring_color, 0.5, 2.0);
            let colors = [tint(side), [255, 255, 255]];
            m.fx.burst(
                &mut m.rng,
                px,
                py,
                &colors,
                18,
                110.0,
                0.6,
                -60.0,
                2.0,
                -HALF_PI,
                TAU,
                ParticleShape::Circle,
            );
            if kind == BuffKind::Pixel {
                m.shake = m.shake.max(3.0);
            }
        }
        AbilityKind::SuperJump { impulse } => {
            {
                let p = &mut m.players[side];
                p.vy = -impulse * p.spec().stats.jump;
                p.on_ground = false;
                p.coyote = 0.0;
            }
            let colors = [[217, 208, 183], [255, 255, 255]];
            m.fx.burst(
                &mut m.rng,
                px,
                py + p_ph / 2.0,
                &colors,
                12,
                90.0,
                0.45,
                260.0,
                2.0,
                HALF_PI,
                TAU * 0.8,
                ParticleShape::Rect,
            );
            m.fx.ring(px, py + p_ph / 2.0, 36.0, tint(side), 0.35, 2.0);
        }
        AbilityKind::Dunk { duration, leap } => {
            {
                let p = &mut m.players[side];
                p.buffs.dunk = p.buffs.dunk.max(duration);
                p.dunk_dir = dir;
                p.vy = -leap * p.spec().stats.jump;
                p.vx += dir * 70.0;
                p.on_ground = false;
            }
            let colors = [tint(side), [255, 255, 255]];
            m.fx.burst(
                &mut m.rng,
                px,
                py + p_ph / 2.0,
                &colors,
                14,
                110.0,
                0.5,
                280.0,
                2.0,
                HALF_PI,
                TAU,
                ParticleShape::Rect,
            );
        }
        AbilityKind::Teleport => {
            let target = clamp(m.ball.x - dir * 34.0, 0.0, W);
            let colors = [tint(side), [180, 75, 255]];
            m.fx.burst(
                &mut m.rng,
                px,
                py,
                &colors,
                20,
                140.0,
                0.5,
                0.0,
                2.0,
                -HALF_PI,
                TAU,
                ParticleShape::Circle,
            );
            let free_x = find_free_x(m, side, target);
            {
                let p = &mut m.players[side];
                p.x = free_x;
                p.y = GROUND_Y - p.h / 2.0;
                p.vx = 0.0;
                p.vy = 0.0;
                p.on_ground = true;
                p.facing = dir;
            }
            let (nx, ny) = (m.players[side].x, m.players[side].y);
            let colors = [tint(side), [255, 255, 255]];
            m.fx.burst(
                &mut m.rng,
                nx,
                ny,
                &colors,
                20,
                150.0,
                0.6,
                0.0,
                2.0,
                -HALF_PI,
                TAU,
                ParticleShape::Circle,
            );
            m.fx.ring(nx, ny, 38.0, tint(side), 0.4, 2.0);
        }
        AbilityKind::Reverse { force } => {
            m.ball.vx = dir * force;
            m.ball.vy = m.ball.vy.min(-175.0);
            m.ball.last_toucher = Some(side);
            let bx = m.ball.x;
            let by = m.ball.y;
            m.fx.ring(bx, by, 48.0, tint(side), 0.45, 3.0);
            m.fx.beam(px, py - p_ph * 0.4, bx, by, tint(side), 0.25, 2.0, false);
            let colors = [tint(side), [255, 255, 255]];
            m.fx.burst(
                &mut m.rng,
                bx,
                by,
                &colors,
                14,
                100.0,
                0.5,
                0.0,
                2.0,
                -HALF_PI,
                TAU,
                ParticleShape::Circle,
            );
        }
        AbilityKind::Wave {
            radius,
            speed,
            knock,
        } => {
            let id = m.next_id();
            m.entities.push(Entity {
                id,
                owner: side,
                side,
                x: px,
                y: py - 6.0,
                vx: 0.0,
                vy: 0.0,
                w: 0.0,
                h: 0.0,
                life: radius / speed,
                max_life: radius / speed,
                dead: false,
                kind: EntityKind::Wave {
                    radius: 6.0,
                    max_radius: radius,
                    speed,
                    knock,
                    age: 0.0,
                    applied_ball: false,
                    applied_opp: false,
                },
            });
        }
        AbilityKind::Geyser { warning, duration } => {
            let x = clamp(m.ball.x, GOAL_DEPTH + 22.0, W - GOAL_DEPTH - 22.0);
            let life = warning + duration;
            let id = m.next_id();
            m.entities.push(Entity {
                id,
                owner: side,
                side,
                x,
                y: GROUND_Y,
                vx: 0.0,
                vy: 0.0,
                w: 26.0,
                h: 0.0,
                life,
                max_life: life,
                dead: false,
                kind: EntityKind::Geyser {
                    warning,
                    duration,
                    erupted: false,
                },
            });
        }
        AbilityKind::Zone { radius, duration } => {
            let x = clamp(m.ball.x, 30.0, W - 30.0);
            let id = m.next_id();
            m.entities.push(Entity {
                id,
                owner: side,
                side,
                x,
                y: GROUND_Y,
                vx: 0.0,
                vy: 0.0,
                w: radius * 2.0,
                h: radius,
                life: duration,
                max_life: duration,
                dead: false,
                kind: EntityKind::Zone { radius },
            });
        }
        AbilityKind::BlackHole {
            radius,
            force,
            duration,
        } => {
            let x = clamp(m.ball.x + dir * 10.0, 26.0, W - 26.0);
            let y = clamp(m.ball.y, CEILING_Y + 12.0, GROUND_Y - 10.0);
            let id = m.next_id();
            m.entities.push(Entity {
                id,
                owner: side,
                side,
                x,
                y,
                vx: 0.0,
                vy: 0.0,
                w: radius * 2.0,
                h: radius * 2.0,
                life: duration,
                max_life: duration,
                dead: false,
                kind: EntityKind::BlackHole { radius, force },
            });
            m.shake = m.shake.max(4.0);
        }
        AbilityKind::Disco { duration } => {
            let id = m.next_id();
            m.entities.push(Entity {
                id,
                owner: side,
                side,
                x: px,
                y: py,
                vx: 0.0,
                vy: 0.0,
                w: 76.0,
                h: 32.0,
                life: duration,
                max_life: duration,
                dead: false,
                kind: EntityKind::Disco {
                    radius: 38.0,
                    orb_r: 9.0,
                    angle: 0.0,
                    hit_cd: 0.0,
                },
            });
        }
        AbilityKind::Clone { duration } => {
            let clone_w = pw * 0.85;
            let clone_h = p_ph * 0.85;
            let target = clamp(px + dir * 44.0, 0.0, W);
            let x = find_free_x(m, side, target);
            let char_id = m.players[side].char_id;
            let facing = m.players[side].facing;
            let id = m.next_id();
            m.entities.push(Entity {
                id,
                owner: side,
                side,
                x,
                y: GROUND_Y - clone_h / 2.0,
                vx: 0.0,
                vy: 0.0,
                w: clone_w,
                h: clone_h,
                life: duration,
                max_life: duration,
                dead: false,
                kind: EntityKind::Clone {
                    char_id,
                    facing,
                    on_ground: true,
                    coyote: COYOTE_TIME,
                    hit_cd: 0.0,
                    step: 0.0,
                    prev_jump: false,
                    scale: 0.85,
                },
            });
            let colors = [tint(side), [255, 255, 255]];
            m.fx.burst(
                &mut m.rng,
                x,
                GROUND_Y - 14.0,
                &colors,
                16,
                100.0,
                0.5,
                -40.0,
                2.0,
                -HALF_PI,
                TAU,
                ParticleShape::Circle,
            );
        }
        AbilityKind::Build { kind, duration } => {
            let anchor = if (m.ball.x - px).abs() < 240.0 {
                m.ball.x
            } else {
                px
            };
            let base_x = clamp(
                anchor + own_dir * 22.0,
                GOAL_DEPTH + 18.0,
                W - GOAL_DEPTH - 18.0,
            );
            let (sx, sw, sh, restitution, build_colors): (f32, f32, f32, f32, [[u8; 3]; 3]) =
                match kind {
                    BuildKind::Wall => (
                        base_x,
                        10.0,
                        38.0,
                        0.82,
                        [[255, 201, 61], [255, 138, 61], [246, 241, 230]],
                    ),
                    BuildKind::Tree => (
                        clamp(
                            anchor + own_dir * 38.0,
                            GOAL_DEPTH + 26.0,
                            W - GOAL_DEPTH - 26.0,
                        ),
                        28.0,
                        64.0,
                        0.7,
                        [[127, 207, 106], [189, 240, 169], [79, 154, 63]],
                    ),
                    BuildKind::Signs => (
                        base_x,
                        9.0,
                        27.0,
                        0.7,
                        [[242, 169, 59], [51, 57, 68], [255, 255, 255]],
                    ),
                };
            let mut xs = vec![sx];
            if kind == BuildKind::Signs {
                xs[0] = clamp(base_x - 18.0, GOAL_DEPTH + 12.0, W - GOAL_DEPTH - 12.0);
                let second = clamp(base_x + 18.0, GOAL_DEPTH + 12.0, W - GOAL_DEPTH - 12.0);
                xs.push(second);
            }
            for x in xs {
                let id = m.next_id();
                m.entities.push(Entity {
                    id,
                    owner: side,
                    side,
                    x,
                    y: GROUND_Y,
                    vx: 0.0,
                    vy: 0.0,
                    w: sw,
                    h: sh,
                    life: duration,
                    max_life: duration,
                    dead: false,
                    kind: EntityKind::Solid { kind, restitution },
                });
            }
            m.fx.burst(
                &mut m.rng,
                sx,
                GROUND_Y - sh * 0.5,
                &build_colors,
                if kind == BuildKind::Tree { 26 } else { 14 },
                if kind == BuildKind::Tree {
                    140.0
                } else {
                    100.0
                },
                if kind == BuildKind::Tree { 0.8 } else { 0.5 },
                220.0,
                2.0,
                -HALF_PI,
                TAU,
                ParticleShape::Rect,
            );
            if kind == BuildKind::Tree {
                m.shake = m.shake.max(3.0);
            }
        }
        AbilityKind::Weather { kind, duration } => {
            m.weather = Some(Weather {
                kind,
                timer: duration,
                total: duration,
                owner: side,
            });
            m.refresh_player_size(0);
            m.refresh_player_size(1);
            m.fx.popup("RAIN!", W / 2.0, 90.0, [189, 239, 255], 1.1, 9.0);
            if kind == WeatherKind::Rain {
                m.fx.burst(
                    &mut m.rng,
                    W / 2.0,
                    110.0,
                    &[[120, 190, 230], [180, 230, 255]],
                    18,
                    40.0,
                    0.9,
                    300.0,
                    2.0,
                    HALF_PI,
                    TAU,
                    ParticleShape::Spark,
                );
            }
        }
    }

    true
}
