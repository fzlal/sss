use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbilityCategory {
    Projectile,
    Control,
    Buff,
    Field,
    Instant,
    None,
}

impl AbilityCategory {
    pub fn label(self) -> &'static str {
        match self {
            AbilityCategory::Projectile => "Projectile",
            AbilityCategory::Control => "Control",
            AbilityCategory::Buff => "Buff",
            AbilityCategory::Field => "Field",
            AbilityCategory::Instant => "Instant",
            AbilityCategory::None => "No ability",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjKind {
    Arrow,
    Boomerang,
    Bomb,
    Glove,
    Fireball,
    Bullet,
    Harpoon,
    Hook,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuffKind {
    Sprint,
    Rage,
    Pixel,
    Grow,
    Invisibility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildKind {
    Wall,
    Tree,
    Signs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherKind {
    Rain,
}

#[derive(Clone, Copy, Debug)]
pub enum AbilityKind {
    None,
    Projectile {
        kind: ProjKind,
        speed: f32,
        impulse: f32,
        pull: f32,
    },
    Pull {
        force: f32,
        duration: f32,
    },
    Remote {
        force: f32,
        duration: f32,
    },
    Frost {
        radius: f32,
        duration: f32,
    },
    Shrink {
        range: f32,
        duration: f32,
    },
    Drain {
        range: f32,
        duration: f32,
    },
    Freeze {
        duration: f32,
    },
    Buff {
        kind: BuffKind,
        duration: f32,
    },
    SuperJump {
        impulse: f32,
    },
    Dunk {
        duration: f32,
        leap: f32,
    },
    Teleport,
    Reverse {
        force: f32,
    },
    Wave {
        radius: f32,
        speed: f32,
        knock: f32,
    },
    Geyser {
        warning: f32,
        duration: f32,
    },
    Zone {
        radius: f32,
        duration: f32,
    },
    BlackHole {
        radius: f32,
        force: f32,
        duration: f32,
    },
    Disco {
        duration: f32,
    },
    Clone {
        duration: f32,
    },
    Build {
        kind: BuildKind,
        duration: f32,
    },
    Weather {
        kind: WeatherKind,
        duration: f32,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Ability {
    pub name: &'static str,
    pub desc: &'static str,
    pub category: AbilityCategory,
    pub cost: f32,
    pub cooldown: f32,
    pub kind: AbilityKind,
}

#[derive(Clone, Copy, Debug)]
pub struct CharStats {
    pub speed: f32,
    pub jump: f32,
    pub power: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Character {
    pub id: &'static str,
    pub name: &'static str,
    pub sprite: &'static str,
    pub blurb: &'static str,
    pub stats: CharStats,
    pub ability: Ability,
}

const fn ability(
    name: &'static str,
    desc: &'static str,
    category: AbilityCategory,
    cost: f32,
    cooldown: f32,
    kind: AbilityKind,
) -> Ability {
    Ability {
        name,
        desc,
        category,
        cost,
        cooldown,
        kind,
    }
}

#[allow(clippy::too_many_arguments)]
const fn character(
    id: &'static str,
    name: &'static str,
    sprite: &'static str,
    blurb: &'static str,
    speed: f32,
    jump: f32,
    power: f32,
    ability: Ability,
) -> Character {
    Character {
        id,
        name,
        sprite,
        blurb,
        stats: CharStats { speed, jump, power },
        ability,
    }
}

pub const CHARACTERS: [Character; 35] = [
    character(
        "classic",
        "Classic",
        "spr_slime_classic",
        "The original blob. No tricks, just clean football.",
        1.0,
        1.0,
        1.0,
        ability(
            "No Ability",
            "Pure fundamentals. Every drop of mana stays in the tank.",
            AbilityCategory::None,
            0.0,
            0.0,
            AbilityKind::None,
        ),
    ),
    character(
        "archer",
        "Archer",
        "spr_slime_archer",
        "Keeps one arrow nocked and one eye on the ball.",
        1.0,
        1.0,
        1.05,
        ability(
            "Bow & Arrow",
            "Fires an arrow that sends the ball flying.",
            AbilityCategory::Projectile,
            35.0,
            1.3,
            AbilityKind::Projectile {
                kind: ProjKind::Arrow,
                speed: 380.0,
                impulse: 215.0,
                pull: 0.0,
            },
        ),
    ),
    character(
        "boomer",
        "Boomer",
        "spr_slime_australian",
        "Throws a boomerang. It always comes back. Usually.",
        1.04,
        1.02,
        1.0,
        ability(
            "Boomerang",
            "A curving throw that flies out, turns and comes back for seconds.",
            AbilityCategory::Projectile,
            30.0,
            2.4,
            AbilityKind::Projectile {
                kind: ProjKind::Boomerang,
                speed: 320.0,
                impulse: 200.0,
                pull: 0.0,
            },
        ),
    ),
    character(
        "hoops",
        "Hoops",
        "spr_slime_basket",
        "Joined the wrong sport and refuses to admit it.",
        0.95,
        1.1,
        1.1,
        ability(
            "Dunk",
            "Leaps at the ball and slams it toward goal.",
            AbilityCategory::Buff,
            70.0,
            6.0,
            AbilityKind::Dunk {
                duration: 1.0,
                leap: 360.0,
            },
        ),
    ),
    character(
        "bomb",
        "Bomb",
        "spr_slime_bomb",
        "Handles every disagreement with a controlled explosion.",
        0.95,
        0.95,
        1.1,
        ability(
            "Bomb",
            "Lobs an explosive that blasts the ball away.",
            AbilityCategory::Projectile,
            28.0,
            4.0,
            AbilityKind::Projectile {
                kind: ProjKind::Bomb,
                speed: 300.0,
                impulse: 350.0,
                pull: 0.0,
            },
        ),
    ),
    character(
        "boxer",
        "Boxer",
        "spr_slime_boxer",
        "Sees a ball. Sees a punching bag. Same thing.",
        1.0,
        0.95,
        1.12,
        ability(
            "Boxing Glove",
            "Throws a spring-loaded glove straight through the ball.",
            AbilityCategory::Projectile,
            30.0,
            1.8,
            AbilityKind::Projectile {
                kind: ProjKind::Glove,
                speed: 350.0,
                impulse: 275.0,
                pull: 0.0,
            },
        ),
    ),
    character(
        "builder",
        "Builder",
        "spr_slime_builder",
        "Believes every problem needs more wall.",
        0.95,
        1.0,
        1.05,
        ability(
            "Brick Wall",
            "Builds a temporary wall. Cheap, rude, effective.",
            AbilityCategory::Field,
            2.0,
            3.0,
            AbilityKind::Build {
                kind: BuildKind::Wall,
                duration: 6.0,
            },
        ),
    ),
    character(
        "clock",
        "Clock",
        "spr_slime_clock",
        "Always knows exactly when to waste your time.",
        0.95,
        1.0,
        1.0,
        ability(
            "Time Stop",
            "Freezes the ball and the opponent for a moment. You keep moving.",
            AbilityCategory::Instant,
            70.0,
            12.0,
            AbilityKind::Freeze { duration: 2.8 },
        ),
    ),
    character(
        "cosmos",
        "Cosmos",
        "spr_slime_cosmos",
        "A tiny universe with a surprisingly good left foot.",
        1.0,
        1.0,
        1.0,
        ability(
            "Black Hole",
            "Opens a gravity well that drags the ball off course.",
            AbilityCategory::Field,
            50.0,
            9.0,
            AbilityKind::BlackHole {
                radius: 165.0,
                force: 1050.0,
                duration: 3.0,
            },
        ),
    ),
    character(
        "cowboy",
        "Cowboy",
        "spr_slime_cowboy",
        "Fastest revolver in the western half of the pitch.",
        1.0,
        1.0,
        1.05,
        ability(
            "Revolver",
            "Fires a quick shot that really moves the ball.",
            AbilityCategory::Projectile,
            50.0,
            2.6,
            AbilityKind::Projectile {
                kind: ProjKind::Bullet,
                speed: 650.0,
                impulse: 285.0,
                pull: 0.0,
            },
        ),
    ),
    character(
        "disco",
        "Disco",
        "spr_slime_disco",
        "Scores goals and makes sure everyone sees them.",
        1.0,
        1.0,
        1.0,
        ability(
            "Disco Ball",
            "Spawns a spinning mirror ball that smacks the ball around.",
            AbilityCategory::Field,
            45.0,
            7.0,
            AbilityKind::Disco { duration: 5.0 },
        ),
    ),
    character(
        "diver",
        "Diver",
        "spr_slime_diver",
        "Brings a harpoon to a football match. Rude.",
        0.98,
        1.0,
        1.0,
        ability(
            "Harpoon",
            "Spears the ball and drags it on a taut line.",
            AbilityCategory::Projectile,
            40.0,
            3.5,
            AbilityKind::Projectile {
                kind: ProjKind::Harpoon,
                speed: 480.0,
                impulse: 0.0,
                pull: 0.7,
            },
        ),
    ),
    character(
        "charm",
        "Charm",
        "spr_slime_female",
        "The ball simply likes being near her. Suspicious.",
        1.05,
        1.0,
        0.95,
        ability(
            "Attraction",
            "Beckons the ball toward her for a short moment.",
            AbilityCategory::Control,
            35.0,
            4.5,
            AbilityKind::Pull {
                force: 585.0,
                duration: 1.3,
            },
        ),
    ),
    character(
        "blaze",
        "Blaze",
        "spr_slime_fire",
        "Runs hot. Shoots hotter. Hydration is a myth.",
        1.02,
        1.0,
        1.05,
        ability(
            "Fireball",
            "Launches a sizzling ball of flame at the football.",
            AbilityCategory::Projectile,
            5.0,
            1.4,
            AbilityKind::Projectile {
                kind: ProjKind::Fireball,
                speed: 340.0,
                impulse: 190.0,
                pull: 0.0,
            },
        ),
    ),
    character(
        "fisher",
        "Fisher",
        "spr_slime_fisher",
        "Patient. Methodical. Absolutely will not let go.",
        1.0,
        1.02,
        0.98,
        ability(
            "Fishing Hook",
            "Hooks the ball and reels it toward you.",
            AbilityCategory::Projectile,
            5.0,
            3.0,
            AbilityKind::Projectile {
                kind: ProjKind::Hook,
                speed: 430.0,
                impulse: 0.0,
                pull: 0.55,
            },
        ),
    ),
    character(
        "ghost",
        "Ghost",
        "spr_slime_ghost",
        "Haunts the midfield. Mostly just wants a hug.",
        1.05,
        1.05,
        0.9,
        ability(
            "Invisibility",
            "Fades out of sight for four seconds.",
            AbilityCategory::Buff,
            35.0,
            6.0,
            AbilityKind::Buff {
                kind: BuffKind::Invisibility,
                duration: 4.0,
            },
        ),
    ),
    character(
        "frost",
        "Frost",
        "spr_slime_ice",
        "Plays cool. Very cool. Frosty, even.",
        0.95,
        1.0,
        1.0,
        ability(
            "Frost Beam",
            "A freezing burst that slows the ball and anyone nearby.",
            AbilityCategory::Control,
            5.0,
            2.2,
            AbilityKind::Frost {
                radius: 140.0,
                duration: 2.2,
            },
        ),
    ),
    character(
        "rainmaker",
        "Rainmaker",
        "spr_slime_indian",
        "Brings the weather. Leaves with the ball.",
        1.0,
        1.0,
        1.0,
        ability(
            "Rain Dance",
            "Summons rain that makes the pitch slick for everyone.",
            AbilityCategory::Field,
            35.0,
            8.0,
            AbilityKind::Weather {
                kind: WeatherKind::Rain,
                duration: 7.0,
            },
        ),
    ),
    character(
        "maniac",
        "Maniac",
        "spr_slime_maniac",
        "Tactics: none. Energy: unlimited. Fun: yes.",
        1.08,
        1.02,
        1.08,
        ability(
            "Crazy Mode",
            "Goes absolutely feral: faster and much harder to handle.",
            AbilityCategory::Buff,
            30.0,
            6.0,
            AbilityKind::Buff {
                kind: BuffKind::Rage,
                duration: 5.0,
            },
        ),
    ),
    character(
        "monk",
        "Monk",
        "spr_slime_monk",
        "Inner peace. Outer chaos. Excellent ball control.",
        1.0,
        1.0,
        1.0,
        ability(
            "Telekinesis",
            "Reverses the ball with an unseen shove.",
            AbilityCategory::Instant,
            30.0,
            4.0,
            AbilityKind::Reverse { force: 430.0 },
        ),
    ),
    character(
        "mud",
        "Mud",
        "spr_slime_mud",
        "Was a respectable slime before the first rainy season.",
        0.95,
        1.0,
        1.05,
        ability(
            "Mud Splash",
            "Leaves a sticky puddle that ruins everyone's day.",
            AbilityCategory::Field,
            50.0,
            7.0,
            AbilityKind::Zone {
                radius: 54.0,
                duration: 7.0,
            },
        ),
    ),
    character(
        "reaper",
        "Reaper",
        "spr_slime_murder",
        "Takes your speed. Tips the hat. Scores anyway.",
        1.05,
        1.0,
        0.95,
        ability(
            "Soul Drain",
            "Siphons the opponent's speed and wears it like a scarf.",
            AbilityCategory::Control,
            5.0,
            5.0,
            AbilityKind::Drain {
                range: 235.0,
                duration: 3.0,
            },
        ),
    ),
    character(
        "music",
        "Music",
        "spr_slime_music",
        "Every match has a soundtrack. It is very loud.",
        1.02,
        1.0,
        1.0,
        ability(
            "Sound Wave",
            "Blasts a ring of bass that shoves the ball and opponent.",
            AbilityCategory::Instant,
            35.0,
            5.0,
            AbilityKind::Wave {
                radius: 210.0,
                speed: 560.0,
                knock: 360.0,
            },
        ),
    ),
    character(
        "nature",
        "Nature",
        "spr_slime_nature",
        "Grows a tree. Waits patiently. Blocks your shot.",
        0.95,
        1.0,
        1.05,
        ability(
            "Tree",
            "Grows a huge tree in front of the ball.",
            AbilityCategory::Field,
            90.0,
            12.0,
            AbilityKind::Build {
                kind: BuildKind::Tree,
                duration: 8.0,
            },
        ),
    ),
    character(
        "retro",
        "Retro",
        "spr_slime_retro",
        "Only has 8 bits, but uses every single one.",
        1.0,
        1.0,
        1.05,
        ability(
            "Pixel Power",
            "Overclocks into a chunky, over-powered sprite.",
            AbilityCategory::Buff,
            90.0,
            10.0,
            AbilityKind::Buff {
                kind: BuffKind::Pixel,
                duration: 5.0,
            },
        ),
    ),
    character(
        "robot",
        "Robot",
        "spr_slime_robot",
        "Beep. Calculates ball trajectories. Mostly.",
        0.95,
        0.95,
        1.1,
        ability(
            "Magnet",
            "Magnetically drags the ball into kicking range.",
            AbilityCategory::Control,
            45.0,
            5.5,
            AbilityKind::Pull {
                force: 960.0,
                duration: 1.6,
            },
        ),
    ),
    character(
        "runner",
        "Runner",
        "spr_slime_runner",
        "Has never once been late to a tackle.",
        1.12,
        1.0,
        0.95,
        ability(
            "Sprint",
            "Turns on the jets for four seconds.",
            AbilityCategory::Buff,
            5.0,
            2.5,
            AbilityKind::Buff {
                kind: BuffKind::Sprint,
                duration: 4.0,
            },
        ),
    ),
    character(
        "scientist",
        "Scientist",
        "spr_slime_scientist",
        "Peer-reviewed methods, questionable ethics.",
        1.0,
        1.0,
        0.95,
        ability(
            "Shrink Ray",
            "Temporarily shrinks the opponent. For science.",
            AbilityCategory::Control,
            40.0,
            6.0,
            AbilityKind::Shrink {
                range: 270.0,
                duration: 4.0,
            },
        ),
    ),
    character(
        "sponge",
        "Sponge",
        "spr_slime_sponge",
        "Absorbs pressure, tackles, and the occasional goal.",
        0.92,
        0.98,
        1.08,
        ability(
            "Grow",
            "Inflates into a wall of slime with a huge hit box.",
            AbilityCategory::Buff,
            50.0,
            8.0,
            AbilityKind::Buff {
                kind: BuffKind::Grow,
                duration: 6.0,
            },
        ),
    ),
    character(
        "super",
        "Super",
        "spr_slime_super",
        "Jump first. Ask questions if you land.",
        1.0,
        1.15,
        0.95,
        ability(
            "Super Jump",
            "An extra-high jump with a tiny cape flutter.",
            AbilityCategory::Buff,
            5.0,
            1.8,
            AbilityKind::SuperJump { impulse: 430.0 },
        ),
    ),
    character(
        "traffic",
        "Traffic",
        "spr_slime_traffic",
        "Stops the ball first. Asks questions never.",
        0.98,
        1.0,
        1.05,
        ability(
            "Stop Signs",
            "Plants two temporary signs in the ball's path.",
            AbilityCategory::Field,
            35.0,
            6.0,
            AbilityKind::Build {
                kind: BuildKind::Signs,
                duration: 5.0,
            },
        ),
    ),
    character(
        "tv",
        "TV",
        "spr_slime_tv",
        "Broadcasting live from the center circle.",
        0.95,
        1.0,
        1.0,
        ability(
            "Remote Control",
            "Steers the ball toward the opponent's goal by remote.",
            AbilityCategory::Control,
            40.0,
            5.5,
            AbilityKind::Remote {
                force: 780.0,
                duration: 1.4,
            },
        ),
    ),
    character(
        "twin",
        "Twin",
        "spr_slime_twin",
        "Two slimes. One brain cell. Perfect teamwork.",
        1.0,
        1.0,
        1.0,
        ability(
            "Clone",
            "Spawns a mirror twin that repeats your moves.",
            AbilityCategory::Field,
            40.0,
            8.0,
            AbilityKind::Clone { duration: 7.0 },
        ),
    ),
    character(
        "geyser",
        "Geyser",
        "spr_slime_water",
        "Part slime, part plumbing emergency.",
        1.0,
        1.0,
        1.0,
        ability(
            "Geyser",
            "Erupts under the ball and launches it skyward.",
            AbilityCategory::Field,
            5.0,
            2.8,
            AbilityKind::Geyser {
                warning: 0.35,
                duration: 0.8,
            },
        ),
    ),
    character(
        "alien",
        "Alien",
        "spr_slime_alien",
        "Has never heard of the offside rule. Or Earth.",
        1.05,
        1.05,
        0.95,
        ability(
            "Teleport",
            "Blinks across the pitch in a puff of static.",
            AbilityCategory::Instant,
            40.0,
            5.0,
            AbilityKind::Teleport,
        ),
    ),
];

pub fn by_id(id: &str) -> usize {
    CHARACTERS.iter().position(|c| c.id == id).unwrap_or(0)
}

pub fn random_index(seed: u64) -> usize {
    (seed as usize) % CHARACTERS.len()
}
