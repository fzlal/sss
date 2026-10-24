#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ambient {
    Leaves,
    Dust,
    Sand,
    Bubbles,
    Stars,
    Sparkle,
    Rain,
    Snow,
}

#[derive(Clone, Copy, Debug)]
pub struct Arena {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub background: &'static str,
    pub accent: [u8; 3],
    pub ambient: Ambient,
}

pub const ARENAS: [Arena; 11] = [
    Arena {
        id: "classic",
        name: "Classic Pitch",
        desc: "Fresh grass, bright sky, absolutely no distractions.",
        background: "bg_soccerfield",
        accent: [255, 212, 90],
        ambient: Ambient::Leaves,
    },
    Arena {
        id: "castle",
        name: "Castle Court",
        desc: "A torch-lit stone yard with dramatic acoustics.",
        background: "bg_castle",
        accent: [255, 206, 107],
        ambient: Ambient::Dust,
    },
    Arena {
        id: "desert",
        name: "Dune Derby",
        desc: "Hot sand, hotter tempers. Bring water.",
        background: "bg_desert",
        accent: [255, 135, 72],
        ambient: Ambient::Sand,
    },
    Arena {
        id: "forest",
        name: "Mossy Grove",
        desc: "A quiet woodland clearing that smells like tackles.",
        background: "bg_forest",
        accent: [255, 226, 122],
        ambient: Ambient::Leaves,
    },
    Arena {
        id: "lab",
        name: "Lab 7",
        desc: "Sterile tiles, questionable experiments, excellent lighting.",
        background: "bg_lab",
        accent: [75, 229, 255],
        ambient: Ambient::Bubbles,
    },
    Arena {
        id: "moon",
        name: "Moon Base",
        desc: "A crater-side kickabout under a very dramatic sky.",
        background: "bg_moon",
        accent: [255, 224, 102],
        ambient: Ambient::Stars,
    },
    Arena {
        id: "track",
        name: "Runner Track",
        desc: "All lanes lead to the goal. Probably.",
        background: "bg_runnertrack",
        accent: [255, 210, 63],
        ambient: Ambient::Dust,
    },
    Arena {
        id: "disco",
        name: "Neon Disco",
        desc: "The lights are low, the bass is high, the ball is glowing.",
        background: "bg_disco",
        accent: [75, 229, 255],
        ambient: Ambient::Sparkle,
    },
    Arena {
        id: "underwater",
        name: "Deep Dive",
        desc: "Bubbles, kelp, and a suspiciously heavy football.",
        background: "bg_underwater",
        accent: [140, 247, 229],
        ambient: Ambient::Bubbles,
    },
    Arena {
        id: "urban",
        name: "City Rooftop",
        desc: "A gritty pitch above the traffic. Mind the puddles.",
        background: "bg_urban",
        accent: [255, 184, 77],
        ambient: Ambient::Rain,
    },
    Arena {
        id: "winter",
        name: "Snow Day",
        desc: "Cold feet, warm hearts, a ball that refuses to sit still.",
        background: "bg_winter",
        accent: [255, 122, 138],
        ambient: Ambient::Snow,
    },
];

pub fn arena_index(id: &str) -> usize {
    ARENAS.iter().position(|a| a.id == id).unwrap_or(0)
}
