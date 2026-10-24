use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use super::abilities;
use super::arenas::ARENAS;
use super::characters::{by_id, BuildKind, Character, ProjKind, CHARACTERS};
use super::config::*;
use super::fx::{Fx, ParticleShape};
use super::math::{clamp, dist, Rect};
use super::rng::Rng;

pub const LEFT_GOAL: f32 = GOAL_DEPTH;
pub const RIGHT_GOAL: f32 = W - GOAL_DEPTH;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Countdown,
    Playing,
    Point,
    Ended,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchConfig {
    pub sport: Sport,
    pub arena: usize,
    pub p1: usize,
    pub p2: usize,
    pub play_to: u32,
    pub mana_mode: ManaMode,
    pub seed: u64,
    pub p1_control: f32,
    pub p2_control: f32,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            sport: Sport::Soccer,
            arena: 0,
            p1: 0,
            p2: 4,
            play_to: 6,
            mana_mode: ManaMode::Bonus,
            seed: 1,
            p1_control: 1.0,
            p2_control: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Buffs {
    pub sprint: f32,
    pub rage: f32,
    pub pixel: f32,
    pub grow: f32,
    pub shrink: f32,
    pub invisible: f32,
    pub chill: f32,
    pub drain_slow: f32,
    pub drain_boost: f32,
    pub dunk: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub side: usize,
    pub char_id: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub vx: f32,
    pub vy: f32,
    pub facing: f32,
    pub on_ground: bool,
    pub coyote: f32,
    pub mana: f32,
    pub cooldown: f32,
    pub contact_cd: f32,
    pub buffs: Buffs,
    pub scale: f32,
    pub speed_mul: f32,
    pub power_mul: f32,
    pub jump_mul: f32,
    pub control_factor: f32,
    pub pull_timer: f32,
    pub pull_force: f32,
    pub remote_timer: f32,
    pub remote_force: f32,
    pub drain_timer: f32,
    pub drain_target: Option<usize>,
    pub frozen: bool,
    pub hit_stun: f32,
    pub dunk_dir: f32,
    pub step: f32,
    pub prev_input: InputFrame,
}

impl Player {
    pub fn bottom(&self) -> f32 {
        self.y + self.h / 2.0
    }

    pub fn spec(&self) -> Character {
        CHARACTERS[self.char_id.min(CHARACTERS.len() - 1)]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub r: f32,
    pub spin: f32,
    pub last_toucher: Option<usize>,
    pub chill_timer: f32,
    pub on_ground: bool,
    pub prev_y: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum EntityKind {
    Projectile {
        kind: ProjKind,
        r: f32,
        gravity: f32,
        impulse: f32,
        pull: f32,
        age: f32,
        returning: bool,
        trail: f32,
    },
    Solid {
        kind: BuildKind,
        restitution: f32,
    },
    Wave {
        radius: f32,
        max_radius: f32,
        speed: f32,
        knock: f32,
        age: f32,
        applied_ball: bool,
        applied_opp: bool,
    },
    Geyser {
        warning: f32,
        duration: f32,
        erupted: bool,
    },
    Zone {
        radius: f32,
    },
    BlackHole {
        radius: f32,
        force: f32,
    },
    Disco {
        radius: f32,
        orb_r: f32,
        angle: f32,
        hit_cd: f32,
    },
    Clone {
        char_id: usize,
        facing: f32,
        on_ground: bool,
        coyote: f32,
        hit_cd: f32,
        step: f32,
        prev_jump: bool,
        scale: f32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entity {
    pub id: u32,
    pub owner: usize,
    pub side: usize,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub w: f32,
    pub h: f32,
    pub life: f32,
    pub max_life: f32,
    pub dead: bool,
    pub kind: EntityKind,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Weather {
    pub kind: super::characters::WeatherKind,
    pub timer: f32,
    pub total: f32,
    pub owner: usize,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Freeze {
    pub side: usize,
    pub timer: f32,
    pub total: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ScoreInfo {
    pub side: usize,
    pub own_goal: bool,
    pub scorer: Option<usize>,
    pub score: [u32; 2],
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Stats {
    pub goals: [u32; 2],
    pub hits: [u32; 2],
    pub abilities: [u32; 2],
    pub top_speed: f32,
}

#[derive(Clone, Debug)]
pub enum Event {
    Countdown(u8),
    Go,
    Jump {
        side: usize,
    },
    Kick {
        side: usize,
        strength: f32,
        head: bool,
        clone: bool,
    },
    Bounce {
        strength: f32,
        kind: &'static str,
    },
    Ability {
        side: usize,
        name: &'static str,
    },
    Deny {
        side: usize,
        reason: &'static str,
    },
    Score {
        side: usize,
        own_goal: bool,
        score: [u32; 2],
        pending_end: bool,
        sport: Sport,
    },
    Whistle {
        final_whistle: bool,
    },
    MatchEnd {
        winner: usize,
        score: [u32; 2],
    },
    Explosion,
    Geyser {
        side: usize,
    },
    DiscoHit {
        side: usize,
    },
    ProjectileHit {
        side: usize,
    },
    WaveHit {
        side: usize,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Match {
    pub config: MatchConfig,
    pub sport: Sport,
    pub arena: usize,
    pub rng: Rng,
    pub players: [Player; 2],
    pub ball: Ball,
    pub score: [u32; 2],
    pub stats: Stats,
    pub phase: Phase,
    pub phase_timer: f32,
    pub countdown_total: f32,
    pub countdown_number: u8,
    pub pending_end: bool,
    pub winner: Option<usize>,
    pub go_flash: f32,
    pub score_flash: f32,
    pub entities: Vec<Entity>,
    #[serde(skip)]
    pub fx: Fx,
    #[serde(skip)]
    pub events: Vec<Event>,
    #[serde(skip)]
    pub input_history: [VecDeque<InputFrame>; 2],
    pub weather: Option<Weather>,
    pub freeze: Option<Freeze>,
    pub last_score: Option<ScoreInfo>,
    pub shake: f32,
    pub time: f32,
    pub elapsed: f32,
    pub tick: u64,
    pub serve_side: usize,
    pub entity_id: u32,
    pub mana_mode: ManaMode,
    pub play_to: u32,
}

impl Match {
    pub fn new(config: MatchConfig) -> Self {
        let rng = Rng::new(config.seed);
        let mut m = Self {
            config,
            sport: config.sport,
            arena: config.arena % ARENAS.len(),
            rng,
            players: [
                Self::make_player(0, config.p1, config.p1_control),
                Self::make_player(1, config.p2, config.p2_control),
            ],
            ball: Ball {
                x: W / 2.0,
                y: BALL_START_Y,
                vx: 0.0,
                vy: 0.0,
                r: BALL_RADIUS,
                spin: 0.0,
                last_toucher: None,
                chill_timer: 0.0,
                on_ground: false,
                prev_y: BALL_START_Y,
            },
            score: [0, 0],
            stats: Stats::default(),
            phase: Phase::Countdown,
            phase_timer: COUNTDOWN,
            countdown_total: COUNTDOWN,
            countdown_number: 0,
            pending_end: false,
            winner: None,
            go_flash: 0.0,
            score_flash: 0.0,
            entities: Vec::new(),
            fx: Fx::default(),
            events: Vec::new(),
            input_history: [VecDeque::new(), VecDeque::new()],
            weather: None,
            freeze: None,
            last_score: None,
            shake: 0.0,
            time: 0.0,
            elapsed: 0.0,
            tick: 0,
            serve_side: (config.seed as usize) % 2,
            entity_id: 1,
            mana_mode: config.mana_mode,
            play_to: config.play_to.clamp(1, 15),
        };
        m.start_kickoff(COUNTDOWN, true);
        m
    }

    fn make_player(side: usize, char_id: usize, control_factor: f32) -> Player {
        let char_id = char_id.min(CHARACTERS.len() - 1);
        let mut p = Player {
            side,
            char_id,
            x: W * if side == 0 { 0.27 } else { 0.73 },
            y: GROUND_Y - PLAYER_H / 2.0,
            w: PLAYER_W,
            h: PLAYER_H,
            vx: 0.0,
            vy: 0.0,
            facing: if side == 0 { 1.0 } else { -1.0 },
            on_ground: true,
            coyote: 0.0,
            mana: START_MANA,
            cooldown: 0.0,
            contact_cd: 0.0,
            buffs: Buffs::default(),
            scale: 1.0,
            speed_mul: 1.0,
            power_mul: 1.0,
            jump_mul: 1.0,
            control_factor,
            pull_timer: 0.0,
            pull_force: 0.0,
            remote_timer: 0.0,
            remote_force: 0.0,
            drain_timer: 0.0,
            drain_target: None,
            frozen: false,
            hit_stun: 0.0,
            dunk_dir: if side == 0 { 1.0 } else { -1.0 },
            step: 0.0,
            prev_input: InputFrame::NONE,
        };
        p.x = W * if side == 0 { 0.27 } else { 0.73 };
        p.y = GROUND_Y - p.h / 2.0;
        p
    }

    pub fn next_id(&mut self) -> u32 {
        let id = self.entity_id;
        self.entity_id = self.entity_id.wrapping_add(1);
        id
    }

    pub fn add_event(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    pub fn start_kickoff(&mut self, duration: f32, initial: bool) {
        self.phase = Phase::Countdown;
        self.phase_timer = duration;
        self.countdown_total = duration;
        self.countdown_number = 0;
        self.go_flash = 0.0;
        self.pending_end = false;
        self.winner = None;
        self.freeze = None;
        self.weather = None;
        self.entities.clear();
        self.input_history[0].clear();
        self.input_history[1].clear();
        self.fx.beams.clear();
        self.fx.popups.clear();

        self.ball.x = match self.sport {
            Sport::Volleyball => NET_X + if self.serve_side == 0 { -46.0 } else { 46.0 },
            _ => W / 2.0,
        };
        self.ball.y = match self.sport {
            Sport::Volleyball => 174.0,
            _ => BALL_START_Y,
        };
        self.ball.vx = if self.sport == Sport::Volleyball {
            if self.serve_side == 0 {
                34.0
            } else {
                -34.0
            }
        } else {
            0.0
        };
        self.ball.vy = 0.0;
        self.ball.prev_y = self.ball.y;
        self.ball.spin = 0.0;
        self.ball.chill_timer = 0.0;
        self.ball.last_toucher = None;
        self.ball.on_ground = false;

        for side in 0..2 {
            let mana = if self.mana_mode.resets_on_point() {
                MANA_MAX
            } else if initial {
                START_MANA
            } else {
                self.players[side].mana
            };
            let p = &mut self.players[side];
            p.buffs = Buffs::default();
            p.scale = 1.0;
            p.w = PLAYER_W;
            p.h = PLAYER_H;
            p.x = match self.sport {
                Sport::Volleyball => {
                    if side == 0 {
                        NET_X - 88.0
                    } else {
                        NET_X + 88.0
                    }
                }
                _ => W * if side == 0 { 0.27 } else { 0.73 },
            };
            p.vx = 0.0;
            p.vy = 0.0;
            p.y = GROUND_Y - p.h / 2.0;
            p.facing = if side == 0 { 1.0 } else { -1.0 };
            p.on_ground = true;
            p.coyote = 0.0;
            p.cooldown = 0.0;
            p.contact_cd = 0.0;
            p.pull_timer = 0.0;
            p.remote_timer = 0.0;
            p.drain_timer = 0.0;
            p.drain_target = None;
            p.frozen = false;
            p.hit_stun = 0.0;
            p.prev_input = InputFrame::NONE;
            p.mana = mana;
            self.refresh_player_size(side);
        }
    }

    pub fn refresh_player_size(&mut self, side: usize) {
        let weather_rain =
            matches!(self.weather, Some(w) if w.kind == super::characters::WeatherKind::Rain);
        let p = &mut self.players[side];
        let b = p.buffs;
        let mut scale = 1.0;
        if b.grow > 0.0 {
            scale *= 1.8;
        }
        if b.pixel > 0.0 {
            scale *= 1.5;
        }
        if b.shrink > 0.0 {
            scale *= 0.62;
        }
        scale = clamp(scale, 0.55, 2.1);

        let mut speed = p.spec().stats.speed;
        if b.sprint > 0.0 {
            speed *= 1.55;
        }
        if b.rage > 0.0 {
            speed *= 1.3;
        }
        if b.pixel > 0.0 {
            speed *= 1.25;
        }
        if b.grow > 0.0 {
            speed *= 0.95;
        }
        if b.shrink > 0.0 {
            speed *= 0.82;
        }
        if b.drain_boost > 0.0 {
            speed *= 1.28;
        }
        if b.drain_slow > 0.0 {
            speed *= 0.55;
        }
        if b.chill > 0.0 {
            speed *= 0.62;
        }
        speed *= p.control_factor;
        if weather_rain {
            speed *= 0.92;
        }
        p.speed_mul = speed;

        let mut power = p.spec().stats.power;
        if b.rage > 0.0 {
            power *= 1.35;
        }
        if b.pixel > 0.0 {
            power *= 1.5;
        }
        if b.grow > 0.0 {
            power *= 1.5;
        }
        if b.shrink > 0.0 {
            power *= 0.65;
        }
        p.power_mul = power;
        p.jump_mul = p.spec().stats.jump;

        if (scale - p.scale).abs() > 0.001 {
            let bottom = p.y + p.h / 2.0;
            p.scale = scale;
            p.w = PLAYER_W * scale;
            p.h = PLAYER_H * scale;
            p.y = bottom - p.h / 2.0;
            if p.y + p.h / 2.0 > GROUND_Y {
                p.y = GROUND_Y - p.h / 2.0;
            }
        }
    }

    pub fn update(&mut self, dt: f32, inputs: [InputFrame; 2]) {
        self.time += dt;
        self.go_flash = (self.go_flash - dt).max(0.0);
        self.score_flash = (self.score_flash - dt).max(0.0);
        self.shake = (self.shake - 26.0 * dt).max(0.0);

        match self.phase {
            Phase::Countdown => {
                self.phase_timer -= dt;
                let remain = self.phase_timer.max(0.0);
                let number = if remain > self.countdown_total * 0.66 {
                    3
                } else if remain > self.countdown_total * 0.33 {
                    2
                } else {
                    1
                };
                if number != self.countdown_number {
                    self.countdown_number = number;
                    self.add_event(Event::Countdown(number));
                }
                if self.phase_timer <= 0.0 {
                    self.phase = Phase::Playing;
                    self.go_flash = 0.6;
                    self.add_event(Event::Go);
                }
                self.fx.update(dt);
                return;
            }
            Phase::Point => {
                self.phase_timer -= dt;
                if self.phase_timer <= 0.0 {
                    if self.pending_end {
                        self.phase = Phase::Ended;
                        let winner = self.winner.unwrap_or(0);
                        self.add_event(Event::MatchEnd {
                            winner,
                            score: self.score,
                        });
                    } else {
                        self.start_kickoff(AFTER_POINT_COUNTDOWN, false);
                    }
                }
                self.fx.update(dt);
                return;
            }
            Phase::Ended => {
                self.fx.update(dt);
                return;
            }
            Phase::Playing => {}
        }

        self.elapsed += dt;
        if let Some(freeze) = &mut self.freeze {
            freeze.timer -= dt;
            if freeze.timer <= 0.0 {
                self.freeze = None;
                self.fx
                    .popup("TIME FLOWS", W / 2.0, 104.0, [255, 243, 200], 0.9, 7.0);
            }
        }

        for (side, frame) in inputs.iter().enumerate() {
            let history = &mut self.input_history[side];
            history.push_back(*frame);
            if history.len() > 60 {
                history.pop_front();
            }
            self.players[side].frozen = matches!(self.freeze, Some(f) if f.side != side);
        }
        for side in 0..2 {
            self.update_player_timers(side, dt);
        }
        for (side, frame) in inputs.iter().enumerate() {
            self.update_player(side, dt, *frame);
        }
        self.update_clones(dt);
        self.update_ball(dt);
        if self.phase == Phase::Playing {
            self.update_entities(dt);
        }
        if let Some(weather) = &mut self.weather {
            weather.timer -= dt;
            if weather.timer <= 0.0 {
                self.weather = None;
            }
        }
        self.fx.update(dt);
        self.tick = self.tick.wrapping_add(1);
    }

    fn update_player_timers(&mut self, side: usize, dt: f32) {
        let regen_mode = self.mana_mode;
        let own_half = if side == 0 {
            self.players[side].x < W / 2.0
        } else {
            self.players[side].x > W / 2.0
        };
        let frozen = self.players[side].frozen;
        let p = &mut self.players[side];
        p.contact_cd = (p.contact_cd - dt).max(0.0);
        if frozen {
            return;
        }
        let b = &mut p.buffs;
        for value in [
            &mut b.sprint,
            &mut b.rage,
            &mut b.pixel,
            &mut b.grow,
            &mut b.shrink,
            &mut b.invisible,
            &mut b.chill,
            &mut b.drain_slow,
            &mut b.drain_boost,
            &mut b.dunk,
        ] {
            if *value > 0.0 {
                *value = (*value - dt).max(0.0);
            }
        }
        p.cooldown = (p.cooldown - dt).max(0.0);
        p.pull_timer = (p.pull_timer - dt).max(0.0);
        p.remote_timer = (p.remote_timer - dt).max(0.0);
        p.drain_timer = (p.drain_timer - dt).max(0.0);
        p.hit_stun = (p.hit_stun - dt).max(0.0);
        let regen = regen_mode.regen(own_half);
        p.mana = (p.mana + regen * dt).min(MANA_MAX);
        self.refresh_player_size(side);
    }

    fn update_player(&mut self, side: usize, dt: f32, input: InputFrame) {
        let can_act = self.phase == Phase::Playing && !self.players[side].frozen;
        let want_ability = can_act && input.ability && !self.players[side].prev_input.ability;
        let want_jump = can_act && input.jump && !self.players[side].prev_input.jump;

        if want_ability {
            abilities::activate(self, side);
        }
        if want_jump {
            self.try_jump(side);
        }

        {
            let p = &mut self.players[side];
            p.prev_input = input;
            p.step += dt;
            if p.frozen {
                return;
            }
        }

        let mut move_dir = 0.0;
        if can_act {
            if input.left {
                move_dir -= 1.0;
            }
            if input.right {
                move_dir += 1.0;
            }
        }

        let (p_x, p_bottom) = {
            let p = &self.players[side];
            (p.x, p.bottom())
        };
        let on_mud = self.is_in_mud(p_x) && p_bottom >= GROUND_Y - 2.0;
        let mud_factor = if on_mud { 0.55 } else { 1.0 };
        let (min_x, max_x) = self.player_bounds(side);

        {
            let p = &mut self.players[side];
            let max_speed = PLAYER_MAX_SPEED * p.speed_mul * mud_factor;
            let accel = PLAYER_ACCEL
                * if p.on_ground {
                    1.0
                } else {
                    PLAYER_AIR_ACCEL / PLAYER_ACCEL
                }
                * p.control_factor
                * mud_factor;

            if move_dir != 0.0 {
                p.facing = move_dir;
                p.vx += move_dir * accel * dt;
                if p.vx * move_dir > max_speed {
                    p.vx = move_dir * max_speed;
                }
            } else if p.on_ground {
                p.vx *= (-PLAYER_FRICTION * dt).exp();
                if p.vx.abs() < 4.0 {
                    p.vx = 0.0;
                }
            } else {
                p.vx *= (-PLAYER_AIR_DRAG * dt).exp();
            }
            if p.vx.abs() > max_speed && p.vx * move_dir <= 0.0 {
                p.vx *= (-3.4 * dt).exp();
            }

            if !p.on_ground {
                p.vy += PLAYER_GRAVITY * dt;
                if p.vy > PLAYER_MAX_FALL {
                    p.vy = PLAYER_MAX_FALL;
                }
                p.coyote = (p.coyote - dt).max(0.0);
            }

            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.on_ground = false;
            p.x = clamp(p.x, min_x, max_x);
        }
        self.collide_actor_solids(side);

        let (sx, hard_landing) = {
            let p = &mut self.players[side];
            let hard = p.bottom() >= GROUND_Y && p.vy > 320.0;
            if p.bottom() >= GROUND_Y {
                p.y = GROUND_Y - p.h / 2.0;
                p.vy = 0.0;
                p.on_ground = true;
                p.coyote = COYOTE_TIME;
            }
            (p.x, hard)
        };
        if hard_landing {
            let colors = [[217, 208, 183], [255, 255, 255]];
            self.fx.burst(
                &mut self.rng,
                sx,
                GROUND_Y - 1.0,
                &colors,
                6,
                50.0,
                0.3,
                100.0,
                1.5,
                std::f32::consts::PI,
                std::f32::consts::PI * 0.7,
                ParticleShape::Rect,
            );
        }

        if move_dir == 0.0 {
            let dx = self.ball.x - sx;
            let p = &mut self.players[side];
            if p.on_ground && dx.abs() > 6.0 {
                p.facing = if dx > 0.0 { 1.0 } else { -1.0 };
            }
        }
    }

    fn player_bounds(&self, side: usize) -> (f32, f32) {
        let p = &self.players[side];
        let half = p.w / 2.0;
        match self.sport {
            Sport::Volleyball => {
                if side == 0 {
                    (half + 6.0, NET_X - half - 7.0)
                } else {
                    (NET_X + half + 7.0, W - half - 6.0)
                }
            }
            Sport::Soccer => (GOAL_DEPTH + half, W - GOAL_DEPTH - half),
            Sport::Basketball => (half + 10.0, W - half - 10.0),
        }
    }

    fn try_jump(&mut self, side: usize) {
        let p = &mut self.players[side];
        if p.frozen || (!p.on_ground && p.coyote <= 0.0) {
            return;
        }
        p.vy = -PLAYER_JUMP * p.spec().stats.jump;
        p.on_ground = false;
        p.coyote = 0.0;
        let x = p.x;
        let y = p.y + p.h / 2.0;
        self.add_event(Event::Jump { side });
        self.fx.burst(
            &mut self.rng,
            x,
            y,
            &[[240, 240, 240], [255, 255, 255]],
            5,
            45.0,
            0.3,
            140.0,
            1.5,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI * 0.7,
            ParticleShape::Rect,
        );
    }

    fn collide_actor_solids(&mut self, side: usize) {
        let solids: Vec<(Rect, f32)> = self
            .entities
            .iter()
            .filter(|e| matches!(e.kind, EntityKind::Solid { .. }) && e.life > 0.0)
            .map(|e| (solid_rect(e), 0.0))
            .collect();
        for (rect, _) in solids {
            let p = &mut self.players[side];
            let left = p.x - p.w / 2.0;
            let right = p.x + p.w / 2.0;
            let top = p.y - p.h / 2.0;
            let bottom = p.y + p.h / 2.0;
            if right <= rect.x
                || left >= rect.x + rect.w
                || bottom <= rect.y
                || top >= rect.y + rect.h
            {
                continue;
            }
            let overlap_x = (right - rect.x).min(rect.x + rect.w - left);
            let overlap_y = (bottom - rect.y).min(rect.y + rect.h - top);
            if overlap_x < overlap_y {
                if p.x < rect.center_x() {
                    p.x = rect.x - p.w / 2.0;
                    if p.vx > 0.0 {
                        p.vx = 0.0;
                    }
                } else {
                    p.x = rect.x + rect.w + p.w / 2.0;
                    if p.vx < 0.0 {
                        p.vx = 0.0;
                    }
                }
            } else if p.y < rect.center_y() {
                p.y = rect.y - p.h / 2.0;
                if p.vy > 0.0 {
                    p.vy = 0.0;
                }
                p.on_ground = true;
                p.coyote = COYOTE_TIME;
            } else {
                p.y = rect.y + rect.h + p.h / 2.0;
                if p.vy < 0.0 {
                    p.vy = 0.0;
                }
            }
        }
    }

    fn update_clones(&mut self, dt: f32) {
        let len = self.entities.len();
        for i in 0..len {
            let (
                owner,
                side,
                mut x,
                mut y,
                mut vx,
                mut vy,
                mut life,
                mut facing,
                mut on_ground,
                mut coyote,
                mut hit_cd,
                mut step,
                mut prev_jump,
                scale,
            ) = match &self.entities[i].kind {
                EntityKind::Clone {
                    char_id: _,
                    facing,
                    on_ground,
                    coyote,
                    hit_cd,
                    step,
                    prev_jump,
                    scale,
                } => (
                    self.entities[i].owner,
                    self.entities[i].side,
                    self.entities[i].x,
                    self.entities[i].y,
                    self.entities[i].vx,
                    self.entities[i].vy,
                    self.entities[i].life,
                    *facing,
                    *on_ground,
                    *coyote,
                    *hit_cd,
                    *step,
                    *prev_jump,
                    *scale,
                ),
                _ => continue,
            };
            life -= dt;
            hit_cd = (hit_cd - dt).max(0.0);
            step += dt;
            let owner_speed = self.players[owner].speed_mul;
            let idx = self.input_history[side].len().saturating_sub(10);
            let input = self.input_history[side]
                .get(idx)
                .copied()
                .unwrap_or(InputFrame::NONE);
            let mut move_dir = 0.0;
            if input.left {
                move_dir -= 1.0;
            }
            if input.right {
                move_dir += 1.0;
            }
            if move_dir != 0.0 {
                facing = move_dir;
            }
            let mud = self.is_in_mud(x) && y + self.entities[i].h / 2.0 >= GROUND_Y - 2.0;
            let max_speed = PLAYER_MAX_SPEED * owner_speed * 0.82 * if mud { 0.55 } else { 1.0 };
            let accel = PLAYER_ACCEL * 0.85;
            if move_dir != 0.0 {
                vx += move_dir * accel * dt;
                if vx * move_dir > max_speed {
                    vx = move_dir * max_speed;
                }
            } else if on_ground {
                vx *= (-PLAYER_FRICTION * dt).exp();
            } else {
                vx *= (-PLAYER_AIR_DRAG * dt).exp();
            }
            if input.jump && !prev_jump && on_ground {
                vy = -PLAYER_JUMP * 0.92;
                on_ground = false;
                let colors = [[255, 255, 255], [230, 230, 230]];
                self.fx.burst(
                    &mut self.rng,
                    x,
                    y + self.entities[i].h / 2.0,
                    &colors,
                    4,
                    40.0,
                    0.25,
                    120.0,
                    1.5,
                    std::f32::consts::FRAC_PI_2,
                    std::f32::consts::PI * 0.6,
                    ParticleShape::Rect,
                );
            }
            prev_jump = input.jump;
            if !on_ground {
                vy += PLAYER_GRAVITY * dt;
                vy = vy.min(PLAYER_MAX_FALL);
                coyote = (coyote - dt).max(0.0);
            }
            x += vx * dt;
            y += vy * dt;
            on_ground = false;

            let w = self.entities[i].w;
            let (min_x, max_x) = if self.sport == Sport::Volleyball {
                let half = w / 2.0;
                if side == 0 {
                    (half + 6.0, NET_X - half - 7.0)
                } else {
                    (NET_X + half + 7.0, W - half - 6.0)
                }
            } else {
                (w / 2.0 + 10.0, W - w / 2.0 - 10.0)
            };
            x = clamp(x, min_x, max_x);

            let solids: Vec<Rect> = self
                .entities
                .iter()
                .filter(|e| matches!(e.kind, EntityKind::Solid { .. }) && e.life > 0.0)
                .map(solid_rect)
                .collect();
            let h = self.entities[i].h;
            for rect in solids {
                let left = x - w / 2.0;
                let right = x + w / 2.0;
                let top = y - h / 2.0;
                let bottom = y + h / 2.0;
                if right <= rect.x
                    || left >= rect.x + rect.w
                    || bottom <= rect.y
                    || top >= rect.y + rect.h
                {
                    continue;
                }
                let overlap_x = (right - rect.x).min(rect.x + rect.w - left);
                let overlap_y = (bottom - rect.y).min(rect.y + rect.h - top);
                if overlap_x < overlap_y {
                    x = if x < rect.center_x() {
                        rect.x - w / 2.0
                    } else {
                        rect.x + rect.w + w / 2.0
                    };
                    vx = 0.0;
                } else if y < rect.center_y() {
                    y = rect.y - h / 2.0;
                    vy = 0.0;
                    on_ground = true;
                } else {
                    y = rect.y + rect.h + h / 2.0;
                    vy = 0.0;
                }
            }
            if y + h / 2.0 >= GROUND_Y {
                y = GROUND_Y - h / 2.0;
                vy = 0.0;
                on_ground = true;
                coyote = COYOTE_TIME;
            }

            let e = &mut self.entities[i];
            e.x = x;
            e.y = y;
            e.vx = vx;
            e.vy = vy;
            e.life = life;
            if let EntityKind::Clone {
                facing: f,
                on_ground: og,
                coyote: c,
                hit_cd: hc,
                step: s,
                prev_jump: pj,
                ..
            } = &mut e.kind
            {
                *f = facing;
                *og = on_ground;
                *c = coyote;
                *hc = hit_cd;
                *s = step;
                *pj = prev_jump;
                let _ = scale;
            }
        }
    }

    fn update_ball(&mut self, dt: f32) {
        self.ball.chill_timer = (self.ball.chill_timer - dt).max(0.0);

        for side in 0..2 {
            let remote = self.players[side].remote_timer > 0.0;
            let remote_force = self.players[side].remote_force;
            let pull = self.players[side].pull_timer > 0.0;
            let pull_force = self.players[side].pull_force;
            if remote {
                let target_x = if side == 0 {
                    RIGHT_GOAL + 8.0
                } else {
                    LEFT_GOAL - 8.0
                };
                let target_y = GROUND_Y - 36.0;
                let dx = target_x - self.ball.x;
                let dy = target_y - self.ball.y;
                let len = (dx * dx + dy * dy).sqrt().max(1.0);
                self.ball.vx += (dx / len) * remote_force * dt;
                self.ball.vy += (dy / len) * remote_force * 0.55 * dt;
                if self.rng.chance(dt * 8.0) {
                    self.fx.particle(super::fx::Particle {
                        x: self.ball.x,
                        y: self.ball.y,
                        vx: -self.ball.vx * 0.1,
                        vy: -self.ball.vy * 0.1,
                        life: 0.25,
                        max_life: 0.25,
                        size: 1.5,
                        color: [75, 229, 255],
                        gravity: 0.0,
                        shape: ParticleShape::Circle,
                        fade: true,
                        spin: 0.0,
                        rot: 0.0,
                    });
                }
            }
            if pull {
                let dx = self.players[side].x - self.ball.x;
                let dy = self.players[side].y - self.ball.y;
                let d = (dx * dx + dy * dy).sqrt().max(1.0);
                if d > 26.0 {
                    self.ball.vx += (dx / d) * pull_force * dt;
                    self.ball.vy += (dy / d) * pull_force * 0.45 * dt;
                }
            }
        }

        if self.freeze.is_none() {
            for e in &self.entities {
                if let EntityKind::BlackHole { radius, force } = e.kind {
                    if e.life <= 0.0 {
                        continue;
                    }
                    let dx = e.x - self.ball.x;
                    let dy = e.y - self.ball.y;
                    let d = (dx * dx + dy * dy).sqrt();
                    if d < radius && d > 2.0 {
                        let strength = force * (1.0 - d / radius);
                        self.ball.vx += (dx / d) * strength * dt + (-dy / d) * strength * 0.16 * dt;
                        self.ball.vy += (dy / d) * strength * dt + (dx / d) * strength * 0.16 * dt;
                    }
                }
            }
            self.ball.vy += BALL_GRAVITY * dt;
            self.ball.vx *= (-BALL_DRAG * dt).exp();
            self.ball.vy *= (-BALL_DRAG * 0.35 * dt).exp();
        } else {
            self.ball.vx *= (-5.0 * dt).exp();
            self.ball.vy *= (-5.0 * dt).exp();
        }
        if self.ball.chill_timer > 0.0 {
            self.ball.vx *= (-4.2 * dt).exp();
            self.ball.vy *= (-2.2 * dt).exp();
        }

        let speed = self.ball.vx.hypot(self.ball.vy);
        if speed > BALL_MAX_SPEED {
            self.ball.vx = self.ball.vx / speed * BALL_MAX_SPEED;
            self.ball.vy = self.ball.vy / speed * BALL_MAX_SPEED;
        }
        if speed > self.stats.top_speed {
            self.stats.top_speed = speed;
        }

        let steps = 4;
        let sdt = dt / steps as f32;
        for _ in 0..steps {
            self.ball.prev_y = self.ball.y;
            self.ball.x += self.ball.vx * sdt;
            self.ball.y += self.ball.vy * sdt;
            self.collide_ball_solids();
            self.collide_ball_sport();
            if self.check_basket() {
                return;
            }
            self.collide_ball_bodies();
            if self.constrain_ball() {
                return;
            }
        }
        self.ball.spin += self.ball.vx * dt * 0.055;
    }

    fn collide_ball_solids(&mut self) {
        let count = self.entities.len();
        for i in 0..count {
            if !matches!(self.entities[i].kind, EntityKind::Solid { .. })
                || self.entities[i].life <= 0.0
            {
                continue;
            }
            let rect = solid_rect(&self.entities[i]);
            let restitution = match self.entities[i].kind {
                EntityKind::Solid { restitution, .. } => restitution,
                _ => 0.7,
            };
            collide_ball_rect(&mut self.ball, &rect, restitution, &mut self.events);
        }
    }

    fn collide_ball_sport(&mut self) {
        match self.sport {
            Sport::Volleyball => self.collide_net(),
            Sport::Basketball => self.collide_basket_geometry(),
            Sport::Soccer => self.collide_goal_frame(),
        }
    }

    fn collide_goal_frame(&mut self) {
        let bar_y = GOAL_TOP - GOAL_BAR_THICKNESS / 2.0;
        let bar_w = GOAL_DEPTH + GOAL_FRAME_OVERHANG;
        for bar_x in [-GOAL_FRAME_OVERHANG, W - GOAL_DEPTH] {
            let bar = Rect::new(bar_x, bar_y, bar_w, GOAL_BAR_THICKNESS);
            collide_ball_rect_kind(&mut self.ball, &bar, 0.82, "post", 45.0, &mut self.events);
        }
    }

    fn collide_net(&mut self) {
        let b = &self.ball;
        if b.y + b.r < NET_TOP {
            return;
        }
        let net_rect = Rect::new(NET_X - 4.0, NET_TOP, 8.0, GROUND_Y - NET_TOP);
        let nx = clamp(b.x, net_rect.x, net_rect.x + net_rect.w);
        let ny = clamp(b.y, net_rect.y, net_rect.y + net_rect.h);
        let dx = b.x - nx;
        let dy = b.y - ny;
        let d = (dx * dx + dy * dy).sqrt();
        if d >= b.r {
            return;
        }
        let normal_x = if d < 0.001 {
            if b.x < NET_X {
                -1.0
            } else {
                1.0
            }
        } else {
            dx / d
        };
        let normal_y = if d < 0.001 { 0.0 } else { dy / d };
        let penetration = b.r - d;
        self.ball.x += normal_x * penetration;
        self.ball.y += normal_y * penetration;
        let vn = self.ball.vx * normal_x + self.ball.vy * normal_y;
        if vn < 0.0 {
            self.ball.vx -= 1.45 * vn * normal_x;
            self.ball.vy -= 1.45 * vn * normal_y;
            self.ball.vx *= 0.82;
            self.add_event(Event::Bounce {
                strength: -vn,
                kind: "net",
            });
        }
    }

    fn collide_basket_geometry(&mut self) {
        for board_x in [BACKBOARD_LEFT_X, BACKBOARD_RIGHT_X] {
            let rect = Rect::new(
                board_x - 4.0,
                BACKBOARD_TOP,
                8.0,
                BACKBOARD_BOTTOM - BACKBOARD_TOP,
            );
            collide_ball_rect(&mut self.ball, &rect, 0.72, &mut self.events);
        }
        for (hoop_x, _) in [(HOOP_LEFT_X, 1usize), (HOOP_RIGHT_X, 0usize)] {
            for post_x in [hoop_x - RIM_HALF_WIDTH, hoop_x + RIM_HALF_WIDTH] {
                let dx = self.ball.x - post_x;
                let dy = self.ball.y - HOOP_Y;
                let d = (dx * dx + dy * dy).sqrt();
                let min_d = self.ball.r + RIM_POST_RADIUS;
                if d < min_d && d > 0.001 {
                    let nx = dx / d;
                    let ny = dy / d;
                    self.ball.x += nx * (min_d - d);
                    self.ball.y += ny * (min_d - d);
                    let vn = self.ball.vx * nx + self.ball.vy * ny;
                    if vn < 0.0 {
                        self.ball.vx -= 1.55 * vn * nx;
                        self.ball.vy -= 1.55 * vn * ny;
                        self.add_event(Event::Bounce {
                            strength: -vn,
                            kind: "rim",
                        });
                    }
                }
            }
        }
    }

    fn check_basket(&mut self) -> bool {
        if self.sport != Sport::Basketball || self.phase != Phase::Playing {
            return false;
        }
        let b = &self.ball;
        let crossed = b.prev_y < HOOP_Y && b.y >= HOOP_Y && b.vy > 0.0;
        if !crossed {
            return false;
        }
        let half_open = 14.0;
        if (b.x - HOOP_LEFT_X).abs() < half_open {
            self.award_point(1, false);
            return true;
        }
        if (b.x - HOOP_RIGHT_X).abs() < half_open {
            self.award_point(0, false);
            return true;
        }
        false
    }

    fn collide_ball_bodies(&mut self) {
        for side in 0..2 {
            self.collide_ball_body_with_player(side);
        }
        let count = self.entities.len();
        for i in 0..count {
            let (owner, x, y, w, h, vx, vy, life) = match &self.entities[i].kind {
                EntityKind::Clone { .. } => (
                    self.entities[i].owner,
                    self.entities[i].x,
                    self.entities[i].y,
                    self.entities[i].w,
                    self.entities[i].h,
                    self.entities[i].vx,
                    self.entities[i].vy,
                    self.entities[i].life,
                ),
                _ => continue,
            };
            if life <= 0.0 {
                continue;
            }
            let rect = Rect::new(x - w / 2.0, y - h / 2.0, w, h);
            let power = self.players[owner].power_mul * 0.78;
            self.collide_ball_body(owner, &rect, vx, vy, power, true);
        }
    }

    fn collide_ball_body_with_player(&mut self, side: usize) {
        let p = &self.players[side];
        if p.contact_cd > 0.0 {
            return;
        }
        let rect = Rect::new(p.x - p.w / 2.0, p.y - p.h / 2.0, p.w, p.h);
        let vx = p.vx;
        let vy = p.vy;
        let power = p.power_mul;
        self.collide_ball_body(side, &rect, vx, vy, power, false);
    }

    fn collide_ball_body(
        &mut self,
        owner: usize,
        rect: &Rect,
        body_vx: f32,
        body_vy: f32,
        power: f32,
        is_clone: bool,
    ) {
        if self.players[owner].contact_cd > 0.0 && !is_clone {
            return;
        }
        let b = &self.ball;
        let nx = clamp(b.x, rect.x, rect.x + rect.w);
        let ny = clamp(b.y, rect.y, rect.y + rect.h);
        let dx = b.x - nx;
        let dy = b.y - ny;
        let d = (dx * dx + dy * dy).sqrt();
        if d >= b.r {
            return;
        }
        let (normal_x, normal_y, penetration) = if d < 0.0001 {
            let left = b.x - rect.x;
            let right = rect.x + rect.w - b.x;
            let top = b.y - rect.y;
            let bottom = rect.y + rect.h - b.y;
            let m = left.min(right).min(top).min(bottom);
            if m == left {
                (-1.0, 0.0, left + b.r)
            } else if m == right {
                (1.0, 0.0, right + b.r)
            } else if m == top {
                (0.0, -1.0, top + b.r)
            } else {
                (0.0, 1.0, bottom + b.r)
            }
        } else {
            (dx / d, dy / d, b.r - d)
        };
        self.ball.x += normal_x * penetration;
        self.ball.y += normal_y * penetration;
        let rvx = self.ball.vx - body_vx;
        let rvy = self.ball.vy - body_vy;
        let vn = rvx * normal_x + rvy * normal_y;
        if vn < 0.0 {
            self.ball.vx -= (1.0 + BALL_BODY_BOUNCE) * vn * normal_x;
            self.ball.vy -= (1.0 + BALL_BODY_BOUNCE) * vn * normal_y;
        }
        if !is_clone {
            self.players[owner].contact_cd = CONTACT_COOLDOWN;
        }
        let body_speed = body_vx.hypot(body_vy);
        let head = normal_y < -0.6;
        let kick = (82.0 + body_speed * 0.8 + if head { 44.0 } else { 0.0 }) * power;
        self.ball.vx += normal_x * kick + body_vx * 0.42;
        self.ball.vy += normal_y * kick + body_vy * 0.22;
        let outward = (self.ball.vx - body_vx) * normal_x + (self.ball.vy - body_vy) * normal_y;
        if outward < 60.0 {
            self.ball.vx += normal_x * (60.0 - outward);
            self.ball.vy += normal_y * (60.0 - outward);
        }
        let bs = self.ball.vx.hypot(self.ball.vy);
        if bs > BALL_MAX_SPEED {
            self.ball.vx = self.ball.vx / bs * BALL_MAX_SPEED;
            self.ball.vy = self.ball.vy / bs * BALL_MAX_SPEED;
        }
        self.ball.last_toucher = Some(owner);
        self.stats.hits[owner] += 1;
        let strength = self.ball.vx.hypot(self.ball.vy);
        self.add_event(Event::Kick {
            side: owner,
            strength,
            head,
            clone: is_clone,
        });
        let bx = self.ball.x;
        let by = self.ball.y;
        self.fx.burst(
            &mut self.rng,
            bx,
            by,
            &[[255, 255, 255], [235, 235, 235]],
            5,
            65.0,
            0.3,
            150.0,
            1.5,
            -std::f32::consts::FRAC_PI_2,
            std::f32::consts::TAU,
            ParticleShape::Circle,
        );

        if !is_clone && self.players[owner].buffs.dunk > 0.0 {
            let dir = self.players[owner].dunk_dir;
            self.players[owner].buffs.dunk = 0.0;
            self.ball.vx = dir * 370.0;
            self.ball.vy = 205.0;
            self.fx
                .popup("SLAM!", bx, by - 16.0, [255, 210, 74], 0.9, 7.0);
            self.fx.burst(
                &mut self.rng,
                bx,
                by,
                &[[255, 210, 74], [255, 255, 255]],
                10,
                120.0,
                0.4,
                200.0,
                2.0,
                -std::f32::consts::FRAC_PI_2,
                std::f32::consts::TAU,
                ParticleShape::Rect,
            );
            self.shake = self.shake.max(3.0);
        }
    }

    fn constrain_ball(&mut self) -> bool {
        if self.ball.y + self.ball.r >= GROUND_Y {
            if self.sport == Sport::Volleyball && self.phase == Phase::Playing {
                let scorer = if self.ball.x < NET_X { 1 } else { 0 };
                self.ball.y = GROUND_Y - self.ball.r;
                self.award_point(scorer, false);
                return true;
            }
            let impact = self.ball.vy;
            self.ball.y = GROUND_Y - self.ball.r;
            if self.ball.vy > 0.0 {
                self.ball.vy = -self.ball.vy * BALL_GROUND_BOUNCE;
                if self.ball.vy.abs() < 42.0 {
                    self.ball.vy = 0.0;
                }
                if impact > 120.0 {
                    self.add_event(Event::Bounce {
                        strength: impact,
                        kind: "ground",
                    });
                }
            }
            let mud = self.is_in_mud(self.ball.x);
            let rain =
                matches!(self.weather, Some(w) if w.kind == super::characters::WeatherKind::Rain);
            let friction = if mud {
                9.0
            } else if rain {
                0.9
            } else {
                BALL_ROLL_FRICTION
            };
            if mud {
                self.ball.vx *= (-9.0f32 / 60.0).exp();
            } else if self.ball.vy == 0.0 {
                self.ball.vx *= (-friction / 60.0).exp();
            }
            if self.ball.vx.abs() < 3.0 {
                self.ball.vx = 0.0;
            }
            self.ball.on_ground = true;
        } else {
            self.ball.on_ground = false;
        }

        if self.ball.y - self.ball.r < CEILING_Y {
            self.ball.y = CEILING_Y + self.ball.r;
            if self.ball.vy < 0.0 {
                self.ball.vy = self.ball.vy.abs() * BALL_WALL_BOUNCE;
            }
        }

        if self.sport == Sport::Soccer {
            if self.ball.y <= GOAL_TOP {
                if self.ball.x - self.ball.r < LEFT_GOAL {
                    self.ball.x = LEFT_GOAL + self.ball.r;
                    if self.ball.vx < 0.0 {
                        self.ball.vx = self.ball.vx.abs() * BALL_WALL_BOUNCE;
                    }
                } else if self.ball.x + self.ball.r > RIGHT_GOAL {
                    self.ball.x = RIGHT_GOAL - self.ball.r;
                    if self.ball.vx > 0.0 {
                        self.ball.vx = -self.ball.vx.abs() * BALL_WALL_BOUNCE;
                    }
                }
            }
            if self.ball.y > GOAL_TOP + 1.0 {
                if self.ball.x + self.ball.r <= LEFT_GOAL {
                    self.award_point(1, false);
                    return true;
                }
                if self.ball.x - self.ball.r >= RIGHT_GOAL {
                    self.award_point(0, false);
                    return true;
                }
            }
        }

        if self.ball.x - self.ball.r < 0.0 {
            self.ball.x = self.ball.r;
            if self.ball.vx < 0.0 {
                self.ball.vx = self.ball.vx.abs() * BALL_WALL_BOUNCE;
            }
        } else if self.ball.x + self.ball.r > W {
            self.ball.x = W - self.ball.r;
            if self.ball.vx > 0.0 {
                self.ball.vx = -self.ball.vx.abs() * BALL_WALL_BOUNCE;
            }
        }
        false
    }

    pub fn award_point(&mut self, side: usize, own_goal_override: bool) {
        if self.phase != Phase::Playing {
            return;
        }
        let scorer = self.ball.last_toucher;
        let own_goal = own_goal_override || scorer == Some(1 - side);
        self.score[side] += 1;
        self.stats.goals[side] += 1;
        self.last_score = Some(ScoreInfo {
            side,
            own_goal,
            scorer,
            score: self.score,
        });
        self.phase = Phase::Point;
        self.phase_timer = POINT_PAUSE;
        self.pending_end = self.score[side] >= self.play_to;
        if self.pending_end {
            self.winner = Some(side);
        }
        self.score_flash = 1.4;
        self.ball.vx = 0.0;
        self.ball.vy = 0.0;
        self.serve_side = 1 - self.serve_side;
        let point_x = match self.sport {
            Sport::Soccer => {
                if side == 0 {
                    RIGHT_GOAL + 10.0
                } else {
                    LEFT_GOAL - 10.0
                }
            }
            Sport::Basketball => {
                if side == 0 {
                    HOOP_RIGHT_X
                } else {
                    HOOP_LEFT_X
                }
            }
            Sport::Volleyball => self.ball.x,
        };
        let point_y = match self.sport {
            Sport::Soccer => GROUND_Y - 80.0,
            Sport::Basketball => HOOP_Y - 10.0,
            Sport::Volleyball => GROUND_Y - 40.0,
        };
        self.fx.confetti(
            &mut self.rng,
            point_x,
            point_y,
            if side == 0 { -1.0 } else { 1.0 },
        );
        self.fx
            .ring(point_x, point_y, 100.0, [255, 213, 74], 0.7, 4.0);
        self.shake = self.shake.max(6.0);
        self.add_event(Event::Score {
            side,
            own_goal,
            score: self.score,
            pending_end: self.pending_end,
            sport: self.sport,
        });
        if self.pending_end {
            self.add_event(Event::Whistle {
                final_whistle: true,
            });
        }
    }

    fn update_entities(&mut self, dt: f32) {
        for e in &mut self.entities {
            e.life -= dt;
        }
        let count = self.entities.len();
        for i in 0..count {
            if self.entities[i].life <= 0.0 {
                if let EntityKind::Projectile {
                    kind: ProjKind::Bomb,
                    ..
                } = self.entities[i].kind
                {
                    let x = self.entities[i].x;
                    let y = self.entities[i].y;
                    let owner = self.entities[i].owner;
                    let side = self.entities[i].side;
                    self.explode(x, y, owner, side, 50.0, 360.0);
                }
                self.entities[i].dead = true;
                continue;
            }
            match self.entities[i].kind {
                EntityKind::Projectile { .. } => self.update_projectile(i, dt),
                EntityKind::Wave { .. } => self.update_wave(i, dt),
                EntityKind::Geyser { .. } => self.update_geyser(i),
                EntityKind::Disco { .. } => self.update_disco(i, dt),
                EntityKind::BlackHole { radius, .. } => {
                    if self.rng.chance(dt * 18.0) {
                        let a = self.rng.range(0.0, std::f32::consts::TAU);
                        let (ex, ey) = (self.entities[i].x, self.entities[i].y);
                        self.fx.particle(super::fx::Particle {
                            x: ex + a.cos() * radius * 0.18,
                            y: ey + a.sin() * radius * 0.16,
                            vx: -a.cos() * 60.0,
                            vy: -a.sin() * 60.0,
                            life: 0.35,
                            max_life: 0.35,
                            size: 2.0,
                            color: if self.rng.chance(0.5) {
                                [184, 174, 255]
                            } else {
                                [255, 224, 102]
                            },
                            gravity: 0.0,
                            shape: ParticleShape::Circle,
                            fade: true,
                            spin: 0.0,
                            rot: 0.0,
                        });
                    }
                }
                EntityKind::Solid { .. } | EntityKind::Zone { .. } | EntityKind::Clone { .. } => {}
            }
        }
        self.entities.retain(|e| e.life > 0.0 && !e.dead);
    }

    fn update_projectile(&mut self, index: usize, dt: f32) {
        let owner = self.entities[index].owner;
        let side = self.entities[index].side;
        let (kind, r, gravity, impulse, pull, mut age, mut returning, mut trail) =
            match self.entities[index].kind {
                EntityKind::Projectile {
                    kind,
                    r,
                    gravity,
                    impulse,
                    pull,
                    age,
                    returning,
                    trail,
                } => (kind, r, gravity, impulse, pull, age, returning, trail),
                _ => return,
            };
        let mut x = self.entities[index].x;
        let mut y = self.entities[index].y;
        let mut vx = self.entities[index].vx;
        let mut vy = self.entities[index].vy;
        age += dt;
        trail = (trail - dt).max(0.0);

        if kind == ProjKind::Boomerang {
            if age > 0.38 {
                returning = true;
            }
            if returning {
                let owner_x = self.players[owner].x;
                let owner_y = self.players[owner].y;
                let dx = owner_x - x;
                let dy = owner_y - y;
                let d = (dx * dx + dy * dy).sqrt().max(1.0);
                vx += (dx / d) * 900.0 * dt;
                vy += (dy / d) * 900.0 * dt;
                let speed = vx.hypot(vy).min(340.0);
                let norm = vx.hypot(vy).max(1.0);
                vx = vx / norm * speed;
                vy = vy / norm * speed;
                if d < 24.0 {
                    self.entities[index].dead = true;
                    return;
                }
            }
            if self.rng.chance(dt * 24.0) {
                self.fx.particle(super::fx::Particle {
                    x,
                    y,
                    vx: -vx * 0.08,
                    vy: -vy * 0.08,
                    life: 0.2,
                    max_life: 0.2,
                    size: 1.5,
                    color: [255, 220, 130],
                    gravity: 0.0,
                    shape: ParticleShape::Circle,
                    fade: true,
                    spin: 0.0,
                    rot: 0.0,
                });
            }
        }

        if matches!(kind, ProjKind::Fireball | ProjKind::Bullet) && trail <= 0.0 {
            trail = if kind == ProjKind::Fireball {
                0.03
            } else {
                0.05
            };
            self.fx.particle(super::fx::Particle {
                x,
                y,
                vx: -vx * 0.06 + self.rng.range(-10.0, 10.0),
                vy: -vy * 0.06 + self.rng.range(-10.0, 10.0),
                life: 0.22,
                max_life: 0.22,
                size: if kind == ProjKind::Fireball { 3.0 } else { 1.5 },
                color: if kind == ProjKind::Fireball {
                    if self.rng.chance(0.5) {
                        [255, 122, 61]
                    } else {
                        [255, 210, 63]
                    }
                } else {
                    [255, 233, 168]
                },
                gravity: 0.0,
                shape: ParticleShape::Circle,
                fade: true,
                spin: 0.0,
                rot: 0.0,
            });
        }

        let steps = if matches!(kind, ProjKind::Bullet | ProjKind::Harpoon | ProjKind::Hook) {
            2
        } else {
            1
        };
        let sdt = dt / steps as f32;
        for _ in 0..steps {
            x += vx * sdt;
            y += vy * sdt;
            if gravity != 0.0 {
                vy += gravity * sdt;
            }
            for solid_idx in 0..self.entities.len() {
                if !matches!(self.entities[solid_idx].kind, EntityKind::Solid { .. })
                    || self.entities[solid_idx].life <= 0.0
                {
                    continue;
                }
                let rect = solid_rect(&self.entities[solid_idx]);
                if x > rect.x - r
                    && x < rect.x + rect.w + r
                    && y > rect.y - r
                    && y < rect.y + rect.h + r
                {
                    if kind == ProjKind::Bomb {
                        self.explode(x, y, owner, side, 50.0, 360.0);
                    } else {
                        self.fx.burst(
                            &mut self.rng,
                            x,
                            y,
                            &[[255, 255, 255], [220, 220, 220]],
                            6,
                            60.0,
                            0.3,
                            120.0,
                            1.5,
                            -std::f32::consts::FRAC_PI_2,
                            std::f32::consts::TAU,
                            ParticleShape::Circle,
                        );
                    }
                    self.entities[index].dead = true;
                    return;
                }
            }

            if dist(x, y, self.ball.x, self.ball.y) < r + self.ball.r {
                if kind == ProjKind::Bomb {
                    self.explode(x, y, owner, side, 52.0, 380.0);
                } else {
                    let len = vx.hypot(vy).max(1.0);
                    let nx = vx / len;
                    let ny = vy / len;
                    if pull > 0.0 {
                        self.players[owner].pull_timer = self.players[owner].pull_timer.max(pull);
                        self.players[owner].pull_force = if kind == ProjKind::Harpoon {
                            960.0
                        } else {
                            780.0
                        };
                        let (ox, oy) = (self.players[owner].x, self.players[owner].y);
                        self.fx.beam(
                            ox,
                            oy - 8.0,
                            self.ball.x,
                            self.ball.y,
                            [120, 220, 255],
                            0.35,
                            2.0,
                            false,
                        );
                    } else {
                        self.ball.vx += nx * impulse;
                        self.ball.vy += ny * impulse - 30.0;
                    }
                    self.ball.last_toucher = Some(owner);
                    let bs = self.ball.vx.hypot(self.ball.vy);
                    if bs > BALL_MAX_SPEED {
                        self.ball.vx = self.ball.vx / bs * BALL_MAX_SPEED;
                        self.ball.vy = self.ball.vy / bs * BALL_MAX_SPEED;
                    }
                    self.fx.burst(
                        &mut self.rng,
                        self.ball.x,
                        self.ball.y,
                        &[[255, 255, 255], [255, 224, 120]],
                        10,
                        110.0,
                        0.4,
                        100.0,
                        2.0,
                        -std::f32::consts::FRAC_PI_2,
                        std::f32::consts::TAU,
                        ParticleShape::Circle,
                    );
                    self.add_event(Event::ProjectileHit { side: owner });
                }
                self.entities[index].dead = true;
                return;
            }

            let opp = 1 - side;
            let (px, py, pw, ph) = (
                self.players[opp].x,
                self.players[opp].y,
                self.players[opp].w,
                self.players[opp].h,
            );
            if x > px - pw / 2.0 - r
                && x < px + pw / 2.0 + r
                && y > py - ph / 2.0 - r
                && y < py + ph / 2.0 + r
            {
                if kind == ProjKind::Bomb {
                    self.explode(x, y, owner, side, 50.0, 360.0);
                } else {
                    let push = vx.signum() * 95.0;
                    self.players[opp].vx += push;
                    self.players[opp].vy = self.players[opp].vy.min(-60.0);
                    self.players[opp].on_ground = false;
                    self.fx.burst(
                        &mut self.rng,
                        x,
                        y,
                        &[[255, 255, 255], [255, 200, 120]],
                        8,
                        80.0,
                        0.35,
                        120.0,
                        2.0,
                        -std::f32::consts::FRAC_PI_2,
                        std::f32::consts::TAU,
                        ParticleShape::Circle,
                    );
                }
                self.entities[index].dead = true;
                return;
            }

            if x < -20.0 || x > W + 20.0 || y > GROUND_Y + 20.0 || y < -20.0 {
                if kind == ProjKind::Bomb {
                    self.explode(x, y, owner, side, 50.0, 360.0);
                }
                self.entities[index].dead = true;
                return;
            }
        }

        let e = &mut self.entities[index];
        e.x = x;
        e.y = y;
        e.vx = vx;
        e.vy = vy;
        if let EntityKind::Projectile {
            age: a,
            returning: r,
            trail: t,
            ..
        } = &mut e.kind
        {
            *a = age;
            *r = returning;
            *t = trail;
        }
    }

    fn update_wave(&mut self, index: usize, dt: f32) {
        let (side, knock, mut radius, speed, mut applied_ball, mut applied_opp) =
            match self.entities[index].kind {
                EntityKind::Wave {
                    radius,
                    speed,
                    knock,
                    applied_ball,
                    applied_opp,
                    ..
                } => (
                    self.entities[index].side,
                    knock,
                    radius,
                    speed,
                    applied_ball,
                    applied_opp,
                ),
                _ => return,
            };
        radius += speed * dt;
        if !applied_ball
            && dist(
                self.entities[index].x,
                self.entities[index].y,
                self.ball.x,
                self.ball.y,
            ) <= radius
        {
            applied_ball = true;
            let dx = self.ball.x - self.entities[index].x;
            let dy = self.ball.y - self.entities[index].y;
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let dir = if side == 0 { 1.0 } else { -1.0 };
            self.ball.vx += (dx / len) * knock * 0.55 + dir * knock * 0.7;
            self.ball.vy += (dy / len) * knock * 0.4 - 95.0;
            self.ball.last_toucher = Some(self.entities[index].owner);
            let bx = self.ball.x;
            let by = self.ball.y;
            self.fx.burst(
                &mut self.rng,
                bx,
                by,
                &[[255, 255, 255], [255, 140, 220]],
                10,
                90.0,
                0.4,
                0.0,
                2.0,
                -std::f32::consts::FRAC_PI_2,
                std::f32::consts::TAU,
                ParticleShape::Circle,
            );
            self.add_event(Event::WaveHit { side });
        }
        let opp = 1 - side;
        if !applied_opp {
            let (ox, oy) = (self.players[opp].x, self.players[opp].y);
            if dist(self.entities[index].x, self.entities[index].y, ox, oy) <= radius {
                applied_opp = true;
                let dx = ox - self.entities[index].x;
                let len = dx.abs().max(1.0);
                self.players[opp].vx += (dx / len) * knock * 0.4;
                self.players[opp].vy = self.players[opp].vy.min(-95.0);
                self.players[opp].on_ground = false;
                self.fx.burst(
                    &mut self.rng,
                    ox,
                    oy,
                    &[[255, 255, 255], [255, 140, 220]],
                    8,
                    80.0,
                    0.4,
                    100.0,
                    2.0,
                    -std::f32::consts::FRAC_PI_2,
                    std::f32::consts::TAU,
                    ParticleShape::Circle,
                );
            }
        }
        if let EntityKind::Wave {
            radius: r,
            applied_ball: ab,
            applied_opp: ao,
            ..
        } = &mut self.entities[index].kind
        {
            *r = radius;
            *ab = applied_ball;
            *ao = applied_opp;
        }
    }

    fn update_geyser(&mut self, index: usize) {
        let (side, warning, duration, mut erupted) = match self.entities[index].kind {
            EntityKind::Geyser {
                warning,
                duration,
                erupted,
            } => (self.entities[index].side, warning, duration, erupted),
            _ => return,
        };
        let elapsed = warning + duration - self.entities[index].life;
        if !erupted && elapsed >= warning {
            erupted = true;
            let x = self.entities[index].x;
            let dir = if side == 0 { 1.0 } else { -1.0 };
            if (self.ball.x - x).abs() < 30.0 && self.ball.y > GROUND_Y - 160.0 {
                self.ball.vy = self.ball.vy.min(-360.0);
                self.ball.vx += dir * 70.0;
                self.ball.last_toucher = Some(self.entities[index].owner);
                self.fx.burst(
                    &mut self.rng,
                    self.ball.x,
                    self.ball.y,
                    &[[189, 246, 255], [123, 224, 255], [255, 255, 255]],
                    22,
                    180.0,
                    0.7,
                    280.0,
                    2.0,
                    -std::f32::consts::FRAC_PI_2,
                    std::f32::consts::PI * 0.6,
                    ParticleShape::Circle,
                );
            }
            for pi in 0..2 {
                if (self.players[pi].x - x).abs() < 24.0
                    && self.players[pi].bottom() >= GROUND_Y - 4.0
                {
                    self.players[pi].vy = -270.0;
                    self.players[pi].on_ground = false;
                }
            }
            self.shake = self.shake.max(2.0);
            self.add_event(Event::Geyser {
                side: self.entities[index].owner,
            });
        }
        if let EntityKind::Geyser { erupted: e, .. } = &mut self.entities[index].kind {
            *e = erupted;
        }
    }

    fn update_disco(&mut self, index: usize, dt: f32) {
        let (owner, side, radius, orb_r, mut angle, mut hit_cd) = match self.entities[index].kind {
            EntityKind::Disco {
                radius,
                orb_r,
                angle,
                hit_cd,
            } => (
                self.entities[index].owner,
                self.entities[index].side,
                radius,
                orb_r,
                angle,
                hit_cd,
            ),
            _ => return,
        };
        angle += dt * 4.4;
        let owner_x = self.players[owner].x;
        let owner_y = self.players[owner].y;
        let x = owner_x + angle.cos() * radius;
        let y = owner_y - 8.0 + angle.sin() * 16.0;
        hit_cd = (hit_cd - dt).max(0.0);
        if hit_cd <= 0.0 && dist(x, y, self.ball.x, self.ball.y) < orb_r + self.ball.r {
            let dir = if side == 0 { 1.0 } else { -1.0 };
            self.ball.vx += dir * 270.0 + angle.cos() * 55.0;
            self.ball.vy -= 160.0;
            self.ball.last_toucher = Some(owner);
            hit_cd = 0.28;
            self.fx.popup(
                "DISCO!",
                self.ball.x,
                self.ball.y - 16.0,
                [255, 217, 74],
                0.8,
                7.0,
            );
            self.fx.burst(
                &mut self.rng,
                self.ball.x,
                self.ball.y,
                &[
                    [255, 217, 74],
                    [255, 93, 241],
                    [75, 229, 255],
                    [255, 255, 255],
                ],
                18,
                130.0,
                0.5,
                0.0,
                2.0,
                -std::f32::consts::FRAC_PI_2,
                std::f32::consts::TAU,
                ParticleShape::Rect,
            );
            self.add_event(Event::DiscoHit { side: owner });
        }
        let e = &mut self.entities[index];
        e.x = x;
        e.y = y;
        if let EntityKind::Disco {
            angle: a,
            hit_cd: hc,
            ..
        } = &mut e.kind
        {
            *a = angle;
            *hc = hit_cd;
        }
    }

    pub fn explode(&mut self, x: f32, y: f32, owner: usize, side: usize, radius: f32, force: f32) {
        let d = dist(x, y, self.ball.x, self.ball.y);
        if d < radius {
            let dx = (self.ball.x - x) / d.max(1.0);
            let dy = (self.ball.y - y) / d.max(1.0);
            let f = force * (1.0 - d / radius);
            self.ball.vx += dx * f;
            self.ball.vy += dy * f - 60.0;
            self.ball.last_toucher = Some(owner);
            let bs = self.ball.vx.hypot(self.ball.vy);
            if bs > BALL_MAX_SPEED {
                self.ball.vx = self.ball.vx / bs * BALL_MAX_SPEED;
                self.ball.vy = self.ball.vy / bs * BALL_MAX_SPEED;
            }
        }
        for pi in 0..2 {
            let pd = dist(x, y, self.players[pi].x, self.players[pi].y);
            if pd < radius + 6.0 && self.players[pi].side != side {
                let dx = (self.players[pi].x - x) / pd.max(1.0);
                self.players[pi].vx += dx * 160.0;
                self.players[pi].vy = self.players[pi].vy.min(-150.0);
                self.players[pi].on_ground = false;
            }
        }
        self.fx.burst(
            &mut self.rng,
            x,
            y,
            &[
                [255, 210, 63],
                [255, 122, 61],
                [255, 77, 77],
                [255, 255, 255],
            ],
            34,
            240.0,
            0.7,
            240.0,
            3.0,
            -std::f32::consts::FRAC_PI_2,
            std::f32::consts::TAU,
            ParticleShape::Rect,
        );
        self.fx.ring(x, y, radius + 18.0, [255, 179, 92], 0.4, 4.0);
        self.shake = self.shake.max(6.0);
        self.add_event(Event::Explosion);
    }

    pub fn is_in_mud(&self, x: f32) -> bool {
        self.entities.iter().any(|e| {
            matches!(e.kind, EntityKind::Zone { .. }) && e.life > 0.0 && (x - e.x).abs() < e.w / 2.0
        })
    }

    pub fn can_use(&self, side: usize) -> bool {
        let p = &self.players[side];
        abilities::block_reason(self, p).is_none()
    }

    pub fn reset_ball_after_pause_at_kickoff(&mut self) {
        self.start_kickoff(AFTER_POINT_COUNTDOWN, false);
    }
}

pub fn solid_rect(entity: &Entity) -> Rect {
    Rect::new(
        entity.x - entity.w / 2.0,
        entity.y - entity.h,
        entity.w,
        entity.h,
    )
}

fn collide_ball_rect(ball: &mut Ball, rect: &Rect, restitution: f32, events: &mut Vec<Event>) {
    collide_ball_rect_kind(ball, rect, restitution, "solid", 110.0, events);
}

fn collide_ball_rect_kind(
    ball: &mut Ball,
    rect: &Rect,
    restitution: f32,
    kind: &'static str,
    min_impact: f32,
    events: &mut Vec<Event>,
) {
    let nx = clamp(ball.x, rect.x, rect.x + rect.w);
    let ny = clamp(ball.y, rect.y, rect.y + rect.h);
    let dx = ball.x - nx;
    let dy = ball.y - ny;
    let d = (dx * dx + dy * dy).sqrt();
    if d >= ball.r {
        return;
    }
    let (normal_x, normal_y, penetration) = if d < 0.0001 {
        let left = ball.x - rect.x;
        let right = rect.x + rect.w - ball.x;
        let top = ball.y - rect.y;
        let bottom = rect.y + rect.h - ball.y;
        let m = left.min(right).min(top).min(bottom);
        if m == left {
            (-1.0, 0.0, left + ball.r)
        } else if m == right {
            (1.0, 0.0, right + ball.r)
        } else if m == top {
            (0.0, -1.0, top + ball.r)
        } else {
            (0.0, 1.0, bottom + ball.r)
        }
    } else {
        (dx / d, dy / d, ball.r - d)
    };
    ball.x += normal_x * penetration;
    ball.y += normal_y * penetration;
    let vn = ball.vx * normal_x + ball.vy * normal_y;
    if vn < 0.0 {
        ball.vx -= (1.0 + restitution) * vn * normal_x;
        ball.vy -= (1.0 + restitution) * vn * normal_y;
        if -vn > min_impact {
            events.push(Event::Bounce {
                strength: -vn,
                kind,
            });
        }
    }
}

pub fn find_free_x(m: &Match, side: usize, target: f32) -> f32 {
    let p = &m.players[side];
    let solids: Vec<Rect> = m
        .entities
        .iter()
        .filter(|e| matches!(e.kind, EntityKind::Solid { .. }) && e.life > 0.0)
        .map(solid_rect)
        .collect();
    let (min_x, max_x) = match m.sport {
        Sport::Volleyball => {
            if side == 0 {
                (p.w / 2.0 + 6.0, NET_X - p.w / 2.0 - 7.0)
            } else {
                (NET_X + p.w / 2.0 + 7.0, W - p.w / 2.0 - 6.0)
            }
        }
        Sport::Soccer => (GOAL_DEPTH + p.w / 2.0, W - GOAL_DEPTH - p.w / 2.0),
        Sport::Basketball => (p.w / 2.0 + 10.0, W - p.w / 2.0 - 10.0),
    };
    for offset in [0.0, -18.0, 18.0, -34.0, 34.0, -50.0, 50.0, -68.0, 68.0] {
        let x = clamp(target + offset, min_x, max_x);
        let rect = Rect::new(x - p.w / 2.0, p.y - p.h / 2.0, p.w, p.h);
        if !solids.iter().any(|s| s.overlaps(&rect)) {
            return x;
        }
    }
    clamp(target, min_x, max_x)
}

pub fn random_character_for(seed: u64) -> usize {
    (seed as usize) % CHARACTERS.len()
}

pub fn character_by_index(index: usize) -> Character {
    CHARACTERS[index.min(CHARACTERS.len() - 1)]
}

pub fn character_id_index(id: &str) -> usize {
    by_id(id)
}
