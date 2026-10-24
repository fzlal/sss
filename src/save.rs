use std::fs;
use std::path::PathBuf;

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::i18n::Language;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameKey(pub u16);

impl GameKey {
    pub const fn from_code(code: u16) -> Self {
        Self(code)
    }

    pub fn pressed(&self) -> bool {
        get_keys_pressed().iter().any(|key| *key as u16 == self.0)
    }

    pub fn down(&self) -> bool {
        get_keys_down().iter().any(|key| *key as u16 == self.0)
    }

    pub fn name(&self) -> String {
        let code = self.0;
        let known = match code {
            0x0020 => Some("SPACE"),
            0x002c => Some("COMMA"),
            0x002e => Some("PERIOD"),
            0xff0d => Some("ENTER"),
            0xff08 => Some("BACKSPACE"),
            0xff09 => Some("TAB"),
            0xff51 => Some("LEFT"),
            0xff52 => Some("UP"),
            0xff53 => Some("RIGHT"),
            0xff54 => Some("DOWN"),
            0xffe1 => Some("LEFT SHIFT"),
            0xffe2 => Some("RIGHT SHIFT"),
            0xffe3 => Some("LEFT CTRL"),
            0xffe4 => Some("RIGHT CTRL"),
            0xffe9 => Some("LEFT ALT"),
            0xffea => Some("RIGHT ALT"),
            0xffbe => Some("F1"),
            0xffbf => Some("F2"),
            0xffc0 => Some("F3"),
            0xffc1 => Some("F4"),
            _ => None,
        };
        if let Some(name) = known {
            return name.to_string();
        }
        let ascii = code as u8 as char;
        if (0x41..=0x5a).contains(&code) {
            return ascii.to_string();
        }
        if (0x30..=0x39).contains(&code) {
            return ascii.to_string();
        }
        format!("KEY {}", code)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Controls {
    pub left: GameKey,
    pub right: GameKey,
    pub jump: GameKey,
    pub ability: GameKey,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            left: GameKey::from_code(0xff51),
            right: GameKey::from_code(0xff53),
            jump: GameKey::from_code(0xff52),
            ability: GameKey::from_code(0x0020),
        }
    }
}

impl Controls {
    pub fn wasd() -> Self {
        Self {
            left: GameKey::from_code(0x41),
            right: GameKey::from_code(0x44),
            jump: GameKey::from_code(0x57),
            ability: GameKey::from_code(0xffe3),
        }
    }

    pub fn arrows_enter() -> Self {
        Self {
            ability: GameKey::from_code(0xff0d),
            ..Self::default()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameSettings {
    pub music: f32,
    pub sfx: f32,
    pub shake: bool,
    pub show_fps: bool,
    #[serde(default)]
    pub language: Language,
    pub p1: Controls,
    pub p2: Controls,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            music: 0.5,
            sfx: 0.75,
            shake: true,
            show_fps: false,
            language: Language::English,
            p1: Controls::default(),
            p2: Controls::wasd(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Progress {
    pub matches: u32,
    pub wins: u32,
    pub championships: u32,
    pub goals: u32,
    pub last_sport: usize,
    pub last_arena: usize,
    pub last_p1: usize,
    pub last_p2: usize,
    pub last_mana_mode: usize,
    pub last_play_to: u32,
    pub last_difficulty: usize,
    pub trophies: Vec<String>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            matches: 0,
            wins: 0,
            championships: 0,
            goals: 0,
            last_sport: 0,
            last_arena: 0,
            last_p1: 0,
            last_p2: 4,
            last_mana_mode: 0,
            last_play_to: 6,
            last_difficulty: 1,
            trophies: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SaveData {
    pub settings: GameSettings,
    pub progress: Progress,
}

const PORTABLE_MARKER: &str = "portable.txt";

fn portable_save_dir(executable: &std::path::Path) -> Option<PathBuf> {
    let dir = executable.parent()?;
    dir.join(PORTABLE_MARKER)
        .is_file()
        .then(|| dir.to_path_buf())
}

pub fn save_dir() -> PathBuf {
    if let Ok(executable) = std::env::current_exe() {
        if let Some(dir) = portable_save_dir(&executable) {
            return dir;
        }
    }
    dirs::data_dir()
        .or_else(dirs::config_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("slime-soccer")
}

pub fn save_path() -> PathBuf {
    save_dir().join("save.json")
}

impl SaveData {
    pub fn load() -> Self {
        let path = save_path();
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn write(&self) -> bool {
        let dir = save_dir();
        if fs::create_dir_all(&dir).is_err() {
            return false;
        }
        let Ok(text) = serde_json::to_string_pretty(self) else {
            return false;
        };
        fs::write(save_path(), text).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_round_trip_keeps_settings() {
        let mut data = SaveData::default();
        data.settings.music = 0.25;
        data.settings.p2.ability = GameKey::from_code(0x5a);
        data.progress.wins = 4;
        let text = serde_json::to_string(&data).unwrap();
        let back: SaveData = serde_json::from_str(&text).unwrap();
        assert!((back.settings.music - 0.25).abs() < 1e-6);
        assert_eq!(back.settings.p2.ability, GameKey::from_code(0x5a));
        assert_eq!(back.progress.wins, 4);
    }

    #[test]
    fn legacy_save_without_language_defaults_to_english() {
        let mut value = serde_json::to_value(SaveData::default()).unwrap();
        value["settings"]
            .as_object_mut()
            .unwrap()
            .remove("language");
        let back: SaveData = serde_json::from_value(value).unwrap();
        assert_eq!(back.settings.language, Language::English);
    }

    #[test]
    fn key_names_are_human_readable() {
        assert_eq!(GameKey::from_code(0x0020).name(), "SPACE");
        assert_eq!(GameKey::from_code(0x41).name(), "A");
        assert_eq!(GameKey::from_code(0xff51).name(), "LEFT");
    }

    #[test]
    fn portable_marker_next_to_executable_selects_its_folder() {
        let dir =
            std::env::temp_dir().join(format!("slime-soccer-portable-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let executable = dir.join("slime-soccer.exe");

        assert_eq!(portable_save_dir(&executable), None);
        fs::write(dir.join(PORTABLE_MARKER), b"portable").unwrap();
        assert_eq!(portable_save_dir(&executable), Some(dir.clone()));

        fs::remove_dir_all(&dir).unwrap();
    }
}
