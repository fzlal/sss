use macroquad::prelude::*;

use crate::game::arenas::ARENAS;
use crate::game::characters::CHARACTERS;

macro_rules! sprite {
    ($name:literal) => {
        Texture2D::from_file_with_format(
            include_bytes!(concat!("../assets/original/sprites/", $name, ".png")),
            Some(ImageFormat::Png),
        )
    };
}

macro_rules! bg {
    ($name:literal) => {
        Texture2D::from_file_with_format(
            include_bytes!(concat!("../assets/original/backgrounds/", $name, ".png")),
            Some(ImageFormat::Png),
        )
    };
}

pub struct Assets {
    pub backgrounds: Vec<Texture2D>,
    pub slimes: Vec<Texture2D>,
    pub ball: Texture2D,
    pub volleyball: Texture2D,
    pub basketball: Texture2D,
    pub goal: Texture2D,
    pub goal_front: Texture2D,
    pub net: Texture2D,
    pub basket: Texture2D,
    pub shadow: Texture2D,
    pub manabar: Texture2D,
    pub manabar_empty: Texture2D,
    pub manabar_back: Texture2D,
    pub manabar_glow: Texture2D,
    pub font: Texture2D,
    pub font_fa: Font,
    pub vs: Texture2D,
    pub back_arrow: Texture2D,
    pub arrow: Texture2D,
    pub boomerang: Texture2D,
    pub glove: Texture2D,
    pub harpoon: Texture2D,
    pub hook: Texture2D,
    pub revolver: Texture2D,
    pub brick: Texture2D,
    pub tree: Texture2D,
    pub stop_sign: Texture2D,
    pub rain_cloud: Texture2D,
    pub rain_drop: Texture2D,
    pub orb: Texture2D,
    pub superstar: Texture2D,
    pub star_hit: Texture2D,
    pub explosion: Texture2D,
    pub ceiling: Texture2D,
    pub icon_cup: Texture2D,
    pub icon_star: Texture2D,
    pub icon_slimes: Texture2D,
    pub icon_shop: Texture2D,
    pub icon_spanner: Texture2D,
    pub icon_book: Texture2D,
    pub icon_cross: Texture2D,
    pub icon_dollar: Texture2D,
    pub icon_random: Texture2D,
    pub choose_slime: Texture2D,
    pub locked: Texture2D,
}

fn tex(bytes: &[u8]) -> Texture2D {
    let texture = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
    texture.set_filter(FilterMode::Nearest);
    texture
}

fn load_slime(name: &str) -> Texture2D {
    let texture = match name {
        "spr_slime_alien" => sprite!("spr_slime_alien"),
        "spr_slime_archer" => sprite!("spr_slime_archer"),
        "spr_slime_australian" => sprite!("spr_slime_australian"),
        "spr_slime_basket" => sprite!("spr_slime_basket"),
        "spr_slime_bomb" => sprite!("spr_slime_bomb"),
        "spr_slime_boxer" => sprite!("spr_slime_boxer"),
        "spr_slime_builder" => sprite!("spr_slime_builder"),
        "spr_slime_clock" => sprite!("spr_slime_clock"),
        "spr_slime_cosmos" => sprite!("spr_slime_cosmos"),
        "spr_slime_cowboy" => sprite!("spr_slime_cowboy"),
        "spr_slime_disco" => sprite!("spr_slime_disco"),
        "spr_slime_diver" => sprite!("spr_slime_diver"),
        "spr_slime_female" => sprite!("spr_slime_female"),
        "spr_slime_fire" => sprite!("spr_slime_fire"),
        "spr_slime_fisher" => sprite!("spr_slime_fisher"),
        "spr_slime_ghost" => sprite!("spr_slime_ghost"),
        "spr_slime_ice" => sprite!("spr_slime_ice"),
        "spr_slime_indian" => sprite!("spr_slime_indian"),
        "spr_slime_maniac" => sprite!("spr_slime_maniac"),
        "spr_slime_monk" => sprite!("spr_slime_monk"),
        "spr_slime_mud" => sprite!("spr_slime_mud"),
        "spr_slime_murder" => sprite!("spr_slime_murder"),
        "spr_slime_music" => sprite!("spr_slime_music"),
        "spr_slime_nature" => sprite!("spr_slime_nature"),
        "spr_slime_retro" => sprite!("spr_slime_retro"),
        "spr_slime_robot" => sprite!("spr_slime_robot"),
        "spr_slime_runner" => sprite!("spr_slime_runner"),
        "spr_slime_scientist" => sprite!("spr_slime_scientist"),
        "spr_slime_sponge" => sprite!("spr_slime_sponge"),
        "spr_slime_super" => sprite!("spr_slime_super"),
        "spr_slime_traffic" => sprite!("spr_slime_traffic"),
        "spr_slime_tv" => sprite!("spr_slime_tv"),
        "spr_slime_twin" => sprite!("spr_slime_twin"),
        "spr_slime_water" => sprite!("spr_slime_water"),
        _ => sprite!("spr_slime_classic"),
    };
    texture.set_filter(FilterMode::Nearest);
    texture
}

fn load_background(name: &str) -> Texture2D {
    let texture = match name {
        "bg_castle" => bg!("bg_castle"),
        "bg_desert" => bg!("bg_desert"),
        "bg_disco" => bg!("bg_disco"),
        "bg_forest" => bg!("bg_forest"),
        "bg_lab" => bg!("bg_lab"),
        "bg_moon" => bg!("bg_moon"),
        "bg_runnertrack" => bg!("bg_runnertrack"),
        "bg_underwater" => bg!("bg_underwater"),
        "bg_urban" => bg!("bg_urban"),
        "bg_winter" => bg!("bg_winter"),
        _ => bg!("bg_soccerfield"),
    };
    texture.set_filter(FilterMode::Nearest);
    texture
}

impl Assets {
    pub fn load() -> Self {
        let backgrounds = ARENAS
            .iter()
            .map(|a| load_background(a.background))
            .collect();
        let slimes = CHARACTERS.iter().map(|c| load_slime(c.sprite)).collect();
        Self {
            backgrounds,
            slimes,
            ball: tex(include_bytes!("../assets/original/sprites/spr_ball.png")),
            volleyball: tex(include_bytes!(
                "../assets/original/sprites/spr_volleyball.png"
            )),
            basketball: tex(include_bytes!(
                "../assets/original/sprites/spr_basketball.png"
            )),
            goal: tex(include_bytes!("../assets/original/sprites/spr_goal.png")),
            goal_front: tex(include_bytes!(
                "../assets/original/sprites/spr_goal_front.png"
            )),
            net: tex(include_bytes!(
                "../assets/original/sprites/spr_volleynet.png"
            )),
            basket: tex(include_bytes!("../assets/original/sprites/spr_basket.png")),
            shadow: tex(include_bytes!("../assets/original/sprites/spr_shadow.png")),
            manabar: tex(include_bytes!("../assets/original/sprites/spr_manabar.png")),
            manabar_empty: tex(include_bytes!(
                "../assets/original/sprites/spr_manabarempty.png"
            )),
            manabar_back: tex(include_bytes!(
                "../assets/original/sprites/spr_manabarback.png"
            )),
            manabar_glow: tex(include_bytes!(
                "../assets/original/sprites/spr_manabarglow.png"
            )),
            font: tex(include_bytes!(
                "../assets/original/sprites/font_atlas_white.png"
            )),
            font_fa: load_ttf_font_from_bytes(include_bytes!(
                "../assets/fonts/IBMPlexSansArabic-Regular.ttf"
            ))
            .expect("embedded Persian font"),
            vs: tex(include_bytes!("../assets/original/sprites/spr_text_vs.png")),
            back_arrow: tex(include_bytes!(
                "../assets/original/sprites/spr_backarrow.png"
            )),
            arrow: tex(include_bytes!("../assets/original/sprites/spr_w_arrow.png")),
            boomerang: tex(include_bytes!(
                "../assets/original/sprites/spr_boomerang.png"
            )),
            glove: tex(include_bytes!(
                "../assets/original/sprites/spr_boxingglove.png"
            )),
            harpoon: tex(include_bytes!("../assets/original/sprites/spr_harpoon.png")),
            hook: tex(include_bytes!(
                "../assets/original/sprites/spr_fishinghook.png"
            )),
            revolver: tex(include_bytes!(
                "../assets/original/sprites/spr_revolver.png"
            )),
            brick: tex(include_bytes!("../assets/original/sprites/spr_brick.png")),
            tree: tex(include_bytes!("../assets/original/sprites/spr_tree.png")),
            stop_sign: tex(include_bytes!(
                "../assets/original/sprites/spr_stopsignsmall.png"
            )),
            rain_cloud: tex(include_bytes!(
                "../assets/original/sprites/spr_raincloud.png"
            )),
            rain_drop: tex(include_bytes!(
                "../assets/original/sprites/spr_raindrop.png"
            )),
            orb: tex(include_bytes!("../assets/original/sprites/spr_orb.png")),
            superstar: tex(include_bytes!(
                "../assets/original/sprites/spr_superstar.png"
            )),
            star_hit: tex(include_bytes!("../assets/original/sprites/spr_starhit.png")),
            explosion: tex(include_bytes!(
                "../assets/original/sprites/spr_explosion.png"
            )),
            ceiling: tex(include_bytes!("../assets/original/sprites/spr_ceiling.png")),
            icon_cup: tex(include_bytes!(
                "../assets/original/sprites/spr_menu_icon_cup.png"
            )),
            icon_star: tex(include_bytes!(
                "../assets/original/sprites/spr_menu_icon_star.png"
            )),
            icon_slimes: tex(include_bytes!(
                "../assets/original/sprites/spr_menu_icon_slimes.png"
            )),
            icon_shop: tex(include_bytes!(
                "../assets/original/sprites/spr_menu_icon_slime_222.png"
            )),
            icon_spanner: tex(include_bytes!(
                "../assets/original/sprites/spr_menu_icon_spanner.png"
            )),
            icon_book: tex(include_bytes!(
                "../assets/original/sprites/spr_menu_icon_book.png"
            )),
            icon_cross: tex(include_bytes!(
                "../assets/original/sprites/spr_menu_icon_cross.png"
            )),
            icon_dollar: tex(include_bytes!(
                "../assets/original/sprites/spr_menu_icon_dollar.png"
            )),
            icon_random: tex(include_bytes!(
                "../assets/original/sprites/spr_slime_random.png"
            )),
            choose_slime: tex(include_bytes!(
                "../assets/original/sprites/spr_menu_slimes_choose1.png"
            )),
            locked: tex(include_bytes!("../assets/original/sprites/spr_locked.png")),
        }
    }

    pub fn slime_for(&self, char_id: usize) -> &Texture2D {
        &self.slimes[char_id.min(self.slimes.len() - 1)]
    }

    pub fn ball_for(&self, sport: crate::game::config::Sport) -> &Texture2D {
        use crate::game::config::Sport;
        match sport {
            Sport::Soccer => &self.ball,
            Sport::Volleyball => &self.volleyball,
            Sport::Basketball => &self.basketball,
        }
    }

    pub fn background_for(&self, arena: usize) -> &Texture2D {
        &self.backgrounds[arena.min(self.backgrounds.len() - 1)]
    }
}
