use super::characters::{AbilityKind, BuffKind, BuildKind, ProjKind};
use super::config::*;
use super::rng::Rng;
use super::sim::{Match, Phase, Player};

pub struct AIController {
    pub side: usize,
    pub difficulty: Difficulty,
    pub params: DifficultyParams,
    rng: Rng,
    think_timer: f32,
    jump_cd: f32,
    ability_cd: f32,
    target_x: f32,
    danger: bool,
    attacking: bool,
    started: bool,
}

impl AIController {
    pub fn new(difficulty: Difficulty, side: usize, seed: u64) -> Self {
        let params = difficulty.params();
        Self {
            side,
            difficulty,
            params,
            rng: Rng::new(
                seed.wrapping_mul(2_654_435_761)
                    .wrapping_add(side as u64 + 1),
            ),
            think_timer: 0.0,
            jump_cd: 0.0,
            ability_cd: 0.45,
            target_x: W / 2.0,
            danger: false,
            attacking: false,
            started: false,
        }
    }

    pub fn reset(&mut self, seed: u64) {
        self.rng = Rng::new(
            seed.wrapping_mul(2_654_435_761)
                .wrapping_add(self.side as u64 + 1),
        );
        self.think_timer = 0.0;
        self.jump_cd = 0.0;
        self.ability_cd = 0.45;
        self.target_x = W / 2.0;
        self.started = false;
    }

    pub fn update(&mut self, m: &Match, dt: f32) -> InputFrame {
        let mut input = InputFrame::NONE;
        self.jump_cd = (self.jump_cd - dt).max(0.0);
        self.ability_cd = (self.ability_cd - dt).max(0.0);
        if m.phase != Phase::Playing {
            self.started = false;
            return input;
        }
        let p = &m.players[self.side];
        if p.frozen {
            return input;
        }
        if !self.started {
            self.started = true;
            self.think(m, p);
        }

        if m.sport == Sport::Volleyball {
            self.think_volleyball(m, p);
            if self.should_jump_volleyball(m, p) {
                input.jump = true;
                self.jump_cd = 0.24 + self.rng.next_f32() * 0.12;
            }
        } else {
            self.think_timer -= dt;
            if self.think_timer <= 0.0 {
                self.think(m, p);
                self.think_timer = self.params.reaction * (0.8 + self.rng.next_f32() * 0.4);
            }

            if self.should_jump(m, p) {
                input.jump = true;
                self.jump_cd = 0.3 + self.rng.next_f32() * 0.14;
            }
        }

        if p.x < self.target_x - 5.0 {
            input.right = true;
        } else if p.x > self.target_x + 5.0 {
            input.left = true;
        }

        if m.sport == Sport::Volleyball {
            let b = &m.ball;
            let own_half = if self.side == 0 {
                b.x < NET_X
            } else {
                b.x > NET_X
            };
            let incoming = if self.side == 0 {
                b.vx < -20.0
            } else {
                b.vx > 20.0
            };
            let contact_zone = (b.x - p.x).abs() < 90.0 && b.y > p.y - 160.0;
            if (own_half || incoming) && contact_zone {
                let dir = if self.side == 0 { 1.0 } else { -1.0 };
                if dir > 0.0 {
                    input.left = false;
                    input.right = true;
                } else {
                    input.right = false;
                    input.left = true;
                }
            }
        }

        if self.ability_cd <= 0.0 && self.should_use_ability(m, p) {
            input.ability = true;
            self.ability_cd = 0.8 + self.rng.next_f32() * 1.1;
        }

        input
    }

    fn think_volleyball(&mut self, m: &Match, p: &Player) {
        let b = &m.ball;
        let side = p.side;
        let net_dir = if side == 0 { 1.0 } else { -1.0 };
        let on_own_half = if side == 0 { b.x < NET_X } else { b.x > NET_X };
        let incoming = if side == 0 { b.vx > 20.0 } else { b.vx < -20.0 };
        let half = p.w / 2.0;
        let (min_x, max_x) = if side == 0 {
            (half + 8.0, NET_X - half - 9.0)
        } else {
            (NET_X + half + 9.0, W - half - 8.0)
        };
        let target = if on_own_half || incoming {
            let (landing_x, _) = volleyball_landing(m);
            let lands_on_own_side = if side == 0 {
                landing_x < NET_X
            } else {
                landing_x > NET_X
            };
            if lands_on_own_side {
                // Stand well behind the landing spot so the ball meets the
                // side of the body; a top contact only pops it straight up.
                landing_x - net_dir * 18.0
            } else {
                NET_X + net_dir * 88.0
            }
        } else {
            NET_X + net_dir * 88.0
        };
        let mut target = target.clamp(min_x, max_x);
        if self.rng.next_f32() < self.params.mistake * 0.5 {
            target += (self.rng.next_f32() - 0.5) * 24.0;
            target = target.clamp(min_x, max_x);
        }
        self.target_x = target;
        self.danger = on_own_half && (target - b.x).abs() < 40.0;
        self.attacking = !on_own_half;
    }

    fn should_jump_volleyball(&mut self, m: &Match, p: &Player) -> bool {
        if !p.on_ground || self.jump_cd > 0.0 {
            return false;
        }
        let b = &m.ball;
        let on_own_half = if p.side == 0 {
            b.x < NET_X
        } else {
            b.x > NET_X
        };
        if !on_own_half {
            return false;
        }
        let dx = b.x - p.x;
        let predicted_y = b.y + b.vy * 0.16 + 0.5 * BALL_GRAVITY * 0.16 * 0.16;
        let in_reach = b.y < p.y - 4.0 || predicted_y < p.y + 8.0;
        let near = dx.abs() < 58.0;
        if near && in_reach {
            return self.rng.next_f32() < 0.35 + self.params.jump_skill * 0.6;
        }
        false
    }

    fn think(&mut self, m: &Match, p: &Player) {
        let b = &m.ball;
        let dir = if p.side == 0 { 1.0 } else { -1.0 };
        let basketball = m.sport == Sport::Basketball;
        let own_goal_x = match (basketball, p.side) {
            (true, 0) => HOOP_LEFT_X,
            (true, _) => HOOP_RIGHT_X,
            (false, 0) => GOAL_DEPTH,
            (false, _) => W - GOAL_DEPTH,
        };
        let opponent_goal_x = match (basketball, p.side) {
            (true, 0) => HOOP_RIGHT_X,
            (true, _) => HOOP_LEFT_X,
            (false, 0) => W - GOAL_DEPTH,
            (false, _) => GOAL_DEPTH,
        };
        let predicted_ball_x = b.x + b.vx * self.params.predict;
        let predict_x = if basketball {
            predicted_ball_x.clamp(BALL_RADIUS, W - BALL_RADIUS)
        } else {
            predicted_ball_x.clamp(GOAL_DEPTH, W - GOAL_DEPTH)
        };
        let ball_own_half = if p.side == 0 {
            b.x < W / 2.0
        } else {
            b.x > W / 2.0
        };
        let moving_to_own_goal = if p.side == 0 {
            b.vx < -20.0
        } else {
            b.vx > 20.0
        };
        self.danger = ball_own_half && moving_to_own_goal;
        let own_third = if p.side == 0 {
            b.x < W * 0.3
        } else {
            b.x > W * 0.7
        };
        let near_own_goal = (b.x - own_goal_x).abs() < 120.0;
        let near_opponent_goal = (b.x - opponent_goal_x).abs() < 165.0;
        self.attacking = !self.danger
            && if p.side == 0 {
                b.x > p.x - 20.0
            } else {
                b.x < p.x + 20.0
            };

        let mut target = predict_x - dir * 31.0;
        if basketball {
            let near_opponent_hoop = (b.x - opponent_goal_x).abs() < 110.0;
            let near_own_hoop = (b.x - own_goal_x).abs() < 110.0;
            target = if near_own_hoop {
                b.x - (b.x - own_goal_x).signum() * 26.0
            } else if near_opponent_hoop {
                b.x - (opponent_goal_x - b.x).signum() * 6.0
            } else if b.y < p.y - 26.0 {
                b.x + b.vx * 0.1
            } else {
                predict_x - dir * 22.0
            };
        } else {
            if self.danger {
                target = predict_x - dir * 42.0;
            }
            if b.y < p.y - 26.0 {
                target = predict_x - dir * 20.0;
            }
            if near_own_goal {
                target = predict_x - dir * 24.0;
            }
            if near_opponent_goal && !self.danger {
                target = predict_x - dir * 36.0;
            }
        }
        let (min_target, max_target) = if basketball {
            (p.w / 2.0 + 10.0, W - p.w / 2.0 - 10.0)
        } else if p.side == 0 {
            (GOAL_DEPTH + 24.0, W - GOAL_DEPTH - 20.0)
        } else {
            (GOAL_DEPTH + 20.0, W - GOAL_DEPTH - 24.0)
        };
        target = target.clamp(min_target, max_target);
        if self.rng.next_f32() < self.params.mistake {
            target += if self.rng.next_f32() < 0.5 { -1.0 } else { 1.0 }
                * (35.0 + self.rng.next_f32() * 65.0);
            target = target.clamp(min_target, max_target);
        }
        if !basketball && own_third && !near_own_goal {
            target = predict_x - dir * 20.0;
        }
        self.target_x = target;
    }

    fn should_jump(&mut self, m: &Match, p: &Player) -> bool {
        if !p.on_ground || self.jump_cd > 0.0 {
            return false;
        }
        let b = &m.ball;
        let dx = b.x - p.x;
        if m.sport == Sport::Basketball {
            let own_hoop_x = if p.side == 0 {
                HOOP_LEFT_X
            } else {
                HOOP_RIGHT_X
            };
            if (b.x - own_hoop_x).abs() < 70.0 {
                return false;
            }
        }
        let predicted_x = b.x + b.vx * 0.12;
        let predicted_y = b.y + b.vy * 0.12 + 0.5 * PLAYER_GRAVITY * 0.12 * 0.12;
        let near = dx.abs() < self.params.jump_window;
        let near_predicted = (predicted_x - p.x).abs() < self.params.jump_window + 8.0;
        let above = b.y < p.y + p.h * 0.5 - 6.0;
        let drop_near = predicted_y > p.y - p.h * 0.5 - 6.0 && predicted_y < p.y + p.h;
        let skill = self.params.jump_skill;

        if above && near && self.rng.next_f32() < 0.25 + skill * 0.65 {
            return true;
        }
        if above && near_predicted && b.vy > -20.0 && self.rng.next_f32() < 0.3 + skill * 0.6 {
            return true;
        }
        if self.danger && dx.abs() < 32.0 && b.y > p.y - 34.0 {
            return true;
        }
        if drop_near && near && self.rng.next_f32() < 0.2 + skill * 0.5 {
            return true;
        }
        false
    }

    fn should_use_ability(&mut self, m: &Match, p: &Player) -> bool {
        let ability = p.spec().ability;
        if matches!(ability.kind, AbilityKind::None) || !m.can_use(self.side) {
            return false;
        }
        let b = &m.ball;
        let opp = &m.players[1 - self.side];
        let dir = if p.side == 0 { 1.0 } else { -1.0 };
        let ball_dist = (p.x - b.x).hypot(p.y - b.y);
        let opp_dist = (p.x - opp.x).hypot(p.y - opp.y);
        let in_front = (b.x - p.x) * dir > -26.0;
        let near_own_goal = if p.side == 0 {
            b.x < 125.0
        } else {
            b.x > W - 125.0
        };
        let near_opponent_goal = if p.side == 0 {
            b.x > W - 180.0
        } else {
            b.x < 180.0
        };
        let attacking_half = if p.side == 0 {
            b.x > W * 0.45
        } else {
            b.x < W * 0.55
        };
        let chance = self.params.ability_chance;

        match ability.kind {
            AbilityKind::None => false,
            AbilityKind::Projectile { kind, .. } => {
                if !in_front || ball_dist > 300.0 {
                    return false;
                }
                match kind {
                    ProjKind::Bomb => near_opponent_goal && self.roll(chance),
                    ProjKind::Harpoon | ProjKind::Hook => ball_dist < 240.0 && self.roll(chance),
                    _ => ball_dist < 280.0 && self.roll(chance),
                }
            }
            AbilityKind::Pull { .. } => ball_dist < 260.0 && ball_dist > 55.0 && self.roll(chance),
            AbilityKind::Remote { .. } => {
                ball_dist < 260.0 && !near_opponent_goal && self.roll(chance)
            }
            AbilityKind::Frost { radius, .. } => {
                (ball_dist < radius + 10.0 || opp_dist < radius - 10.0) && self.roll(chance)
            }
            AbilityKind::Shrink { range, .. } => {
                opp_dist < range && (ball_dist < 170.0 || opp_dist < 120.0) && self.roll(chance)
            }
            AbilityKind::Drain { range, .. } => opp_dist < range && self.roll(chance),
            AbilityKind::Reverse { .. } => {
                (self.danger || near_own_goal) && ball_dist < 300.0 && self.roll(chance)
            }
            AbilityKind::SuperJump { .. } => {
                p.on_ground && b.y < p.y - 40.0 && (b.x - p.x).abs() < 110.0 && self.roll(chance)
            }
            AbilityKind::Dunk { .. } => {
                p.on_ground
                    && near_opponent_goal
                    && ball_dist < 110.0
                    && in_front
                    && self.roll(chance)
            }
            AbilityKind::Buff { kind, .. } => match kind {
                BuffKind::Sprint => ball_dist > 80.0 && self.roll(chance),
                BuffKind::Invisibility => ball_dist > 100.0 && self.roll(chance * 0.35),
                _ => ball_dist < 220.0 && self.roll(chance),
            },
            AbilityKind::Teleport => (b.x - p.x).abs() > 90.0 && self.roll(chance),
            AbilityKind::Freeze { .. } => {
                (self.danger || near_own_goal || near_opponent_goal)
                    && ball_dist < 190.0
                    && self.roll(chance)
            }
            AbilityKind::Wave { radius, .. } => ball_dist < radius * 0.8 && self.roll(chance),
            AbilityKind::Geyser { .. } => {
                ball_dist < 145.0 && b.y > CEILING_Y + 20.0 && self.roll(chance)
            }
            AbilityKind::Zone { .. } => attacking_half && ball_dist < 260.0 && self.roll(chance),
            AbilityKind::BlackHole { .. } => {
                attacking_half && ball_dist < 300.0 && self.roll(chance)
            }
            AbilityKind::Disco { .. } => ball_dist < 220.0 && self.roll(chance),
            AbilityKind::Clone { .. } => ball_dist < 260.0 && !self.danger && self.roll(chance),
            AbilityKind::Build { kind, .. } => match kind {
                BuildKind::Tree => {
                    (self.danger || near_own_goal) && ball_dist < 200.0 && self.roll(chance)
                }
                _ => (self.danger || ball_dist < 170.0) && self.roll(chance),
            },
            AbilityKind::Weather { .. } => attacking_half && self.roll(chance * 0.5),
        }
    }

    fn roll(&mut self, chance: f32) -> bool {
        self.rng.next_f32() < chance
    }
}

fn volleyball_landing(m: &Match) -> (f32, f32) {
    let mut x = m.ball.x;
    let mut y = m.ball.y;
    let mut vx = m.ball.vx;
    let mut vy = m.ball.vy;
    let dt = 1.0 / 120.0;
    let mut elapsed = 0.0;
    for _ in 0..600 {
        vy += BALL_GRAVITY * dt;
        vx *= (-BALL_DRAG * dt).exp();
        vy *= (-BALL_DRAG * 0.35 * dt).exp();
        x += vx * dt;
        y += vy * dt;
        elapsed += dt;

        if y + BALL_RADIUS >= NET_TOP {
            let nx = x.clamp(NET_X - 4.0, NET_X + 4.0);
            let ny = y.clamp(NET_TOP, GROUND_Y);
            let dx = x - nx;
            let dy = y - ny;
            let d = (dx * dx + dy * dy).sqrt();
            if d < BALL_RADIUS && d > 0.001 {
                let normal_x = dx / d;
                let normal_y = dy / d;
                let penetration = BALL_RADIUS - d;
                x += normal_x * penetration;
                y += normal_y * penetration;
                let vn = vx * normal_x + vy * normal_y;
                if vn < 0.0 {
                    vx -= 1.45 * vn * normal_x;
                    vy -= 1.45 * vn * normal_y;
                    vx *= 0.82;
                    vy *= 0.82;
                }
            }
        }

        if y - BALL_RADIUS < CEILING_Y {
            y = CEILING_Y + BALL_RADIUS;
            if vy < 0.0 {
                vy = vy.abs() * BALL_WALL_BOUNCE;
            }
        }
        if x < BALL_RADIUS {
            x = BALL_RADIUS;
            vx = vx.abs() * BALL_WALL_BOUNCE;
        } else if x > W - BALL_RADIUS {
            x = W - BALL_RADIUS;
            vx = -vx.abs() * BALL_WALL_BOUNCE;
        }
        if y + BALL_RADIUS >= GROUND_Y {
            break;
        }
    }
    (x, elapsed)
}
