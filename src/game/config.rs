use serde::{Deserialize, Serialize};

pub const W: f32 = 560.0;
pub const H: f32 = 400.0;

pub const GROUND_Y: f32 = 368.0;
pub const CEILING_Y: f32 = 62.0;
pub const HUD_H: f32 = 58.0;

pub const GOAL_TOP: f32 = 256.0;
pub const GOAL_DEPTH: f32 = 54.0;
pub const GOAL_BAR_THICKNESS: f32 = 7.0;
pub const GOAL_FRAME_OVERHANG: f32 = 16.0;

pub const PLAYER_W: f32 = 46.0;
pub const PLAYER_H: f32 = 26.0;
pub const PLAYER_ACCEL: f32 = 1150.0;
pub const PLAYER_AIR_ACCEL: f32 = 760.0;
pub const PLAYER_MAX_SPEED: f32 = 190.0;
pub const PLAYER_JUMP: f32 = 350.0;
pub const PLAYER_GRAVITY: f32 = 1150.0;
pub const PLAYER_MAX_FALL: f32 = 780.0;
pub const PLAYER_FRICTION: f32 = 9.5;
pub const PLAYER_AIR_DRAG: f32 = 0.6;
pub const COYOTE_TIME: f32 = 0.085;
pub const MANA_MAX: f32 = 100.0;
pub const START_MANA: f32 = 50.0;

pub const BALL_RADIUS: f32 = 9.5;
pub const BALL_GRAVITY: f32 = 630.0;
pub const BALL_DRAG: f32 = 0.16;
pub const BALL_GROUND_BOUNCE: f32 = 0.71;
pub const BALL_WALL_BOUNCE: f32 = 0.74;
pub const BALL_BODY_BOUNCE: f32 = 0.78;
pub const BALL_ROLL_FRICTION: f32 = 1.5;
pub const BALL_MAX_SPEED: f32 = 940.0;
pub const BALL_START_Y: f32 = 196.0;

pub const CONTACT_COOLDOWN: f32 = 0.09;
pub const COUNTDOWN: f32 = 2.4;
pub const POINT_PAUSE: f32 = 2.15;
pub const AFTER_POINT_COUNTDOWN: f32 = 1.5;

pub const NET_X: f32 = W / 2.0;
pub const NET_TOP: f32 = GROUND_Y - 92.0;

pub const HOOP_LEFT_X: f32 = 88.0;
pub const HOOP_RIGHT_X: f32 = W - 88.0;
pub const HOOP_Y: f32 = 238.0;
pub const RIM_HALF_WIDTH: f32 = 22.0;
pub const RIM_POST_RADIUS: f32 = 3.2;
pub const BACKBOARD_LEFT_X: f32 = HOOP_LEFT_X - 39.0;
pub const BACKBOARD_RIGHT_X: f32 = HOOP_RIGHT_X + 39.0;
pub const BACKBOARD_TOP: f32 = HOOP_Y - 36.0;
pub const BACKBOARD_BOTTOM: f32 = HOOP_Y + 44.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sport {
    Soccer,
    Volleyball,
    Basketball,
}

impl Sport {
    pub const ALL: [Sport; 3] = [Sport::Soccer, Sport::Volleyball, Sport::Basketball];

    pub fn key(self) -> &'static str {
        match self {
            Sport::Soccer => "soccer",
            Sport::Volleyball => "volleyball",
            Sport::Basketball => "basketball",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Sport::Soccer => "Soccer",
            Sport::Volleyball => "Volleyball",
            Sport::Basketball => "Basketball",
        }
    }

    pub fn tagline(self) -> &'static str {
        match self {
            Sport::Soccer => "Classic slime football. Two goals, zero rules about dignity.",
            Sport::Volleyball => "Keep the ball off your floor. The net is not a suggestion.",
            Sport::Basketball => "Hoops, headbutts and questionable jump shots.",
        }
    }

    pub fn ball_sprite(self) -> &'static str {
        match self {
            Sport::Soccer => "spr_ball",
            Sport::Volleyball => "spr_volleyball",
            Sport::Basketball => "spr_basketball",
        }
    }

    pub fn point_word(self) -> &'static str {
        match self {
            Sport::Soccer => "GOAL!",
            Sport::Volleyball => "POINT!",
            Sport::Basketball => "BASKET!",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManaMode {
    Bonus,
    Offensive,
    Round,
}

impl ManaMode {
    pub const ALL: [ManaMode; 3] = [ManaMode::Bonus, ManaMode::Offensive, ManaMode::Round];

    pub fn label(self) -> &'static str {
        match self {
            ManaMode::Bonus => "Bonus",
            ManaMode::Offensive => "Offensive",
            ManaMode::Round => "Round",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            ManaMode::Bonus => "Base regen, three times faster in the opponent's half.",
            ManaMode::Offensive => "Charges only while you are in the opponent's half.",
            ManaMode::Round => {
                "Slow regen, faster in the opponent's half; refilled after each point."
            }
        }
    }

    pub fn regen(self, own_half: bool) -> f32 {
        let attacking = !own_half;
        match self {
            ManaMode::Bonus => 6.0 + if attacking { 12.0 } else { 0.0 },
            ManaMode::Offensive => {
                if attacking {
                    11.0
                } else {
                    0.0
                }
            }
            ManaMode::Round => 3.0 + if attacking { 6.0 } else { 0.0 },
        }
    }

    pub fn resets_on_point(self) -> bool {
        self == ManaMode::Round
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard];

    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
        }
    }

    pub fn params(self) -> DifficultyParams {
        match self {
            Difficulty::Easy => DifficultyParams {
                reaction: 0.34,
                speed: 0.78,
                accel: 0.75,
                jump_skill: 0.5,
                jump_window: 58.0,
                ability_chance: 0.3,
                mistake: 0.3,
                predict: 0.12,
            },
            Difficulty::Normal => DifficultyParams {
                reaction: 0.2,
                speed: 0.92,
                accel: 0.9,
                jump_skill: 0.76,
                jump_window: 50.0,
                ability_chance: 0.62,
                mistake: 0.12,
                predict: 0.28,
            },
            Difficulty::Hard => DifficultyParams {
                reaction: 0.09,
                speed: 1.0,
                accel: 1.0,
                jump_skill: 0.95,
                jump_window: 44.0,
                ability_chance: 1.0,
                mistake: 0.03,
                predict: 0.42,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DifficultyParams {
    pub reaction: f32,
    pub speed: f32,
    pub accel: f32,
    pub jump_skill: f32,
    pub jump_window: f32,
    pub ability_chance: f32,
    pub mistake: f32,
    pub predict: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputFrame {
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub ability: bool,
}

impl InputFrame {
    pub const NONE: InputFrame = InputFrame {
        left: false,
        right: false,
        jump: false,
        ability: false,
    };
}
