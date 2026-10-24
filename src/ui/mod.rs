use macroquad::prelude::*;

use crate::assets::Assets;
use crate::audio::{Audio, Sfx};
use crate::game::arenas::ARENAS;
use crate::game::characters::CHARACTERS;
use crate::game::config::{Difficulty, ManaMode, Sport, W};
use crate::game::rng::Rng;
use crate::game::sim::{MatchConfig, Stats};
use crate::i18n;
use crate::net::lan::LanBrowser;
use crate::net::{normalize_address, LanOffer};
use crate::render::{self, Canvas, TextAlign};
use crate::save::{Controls, GameKey, Progress, SaveData};

const TITLE_ITEMS: [&str; 6] = [
    "QUICK MATCH",
    "CHAMPIONSHIP",
    "ONLINE",
    "SETTINGS",
    "HOW TO PLAY",
    "QUIT",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Title,
    Online,
    Setup,
    Roster,
    Arena,
    Settings,
    Controls,
    Help,
    Results,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchMode {
    Casual,
    Championship,
    Online,
}

#[derive(Clone, Debug)]
pub struct LaunchRequest {
    pub config: MatchConfig,
    pub ai: bool,
    pub difficulty: Difficulty,
    pub mode: LaunchMode,
    pub local_side: usize,
}

#[derive(Clone, Debug)]
pub struct MatchReport {
    pub request: LaunchRequest,
    pub score: [u32; 2],
    pub winner: usize,
    pub stats: Stats,
    pub elapsed: f32,
}

pub enum UiAction {
    Launch(LaunchRequest),
    HostOnline(LaunchRequest),
    JoinOnline { addr: String, char_id: usize },
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OnlinePanel {
    Menu,
    Join,
    Lan,
}

#[derive(Clone, Copy, Debug)]
pub struct SetupState {
    pub sport: Sport,
    pub arena: usize,
    pub p1: usize,
    pub p2: usize,
    pub play_to: u32,
    pub mana_mode: ManaMode,
    pub difficulty: Difficulty,
    pub local2p: bool,
}

impl SetupState {
    fn from_progress(progress: &Progress) -> Self {
        Self {
            sport: Sport::ALL[progress.last_sport.min(2)],
            arena: progress.last_arena.min(ARENAS.len() - 1),
            p1: progress.last_p1.min(CHARACTERS.len() - 1),
            p2: progress.last_p2.min(CHARACTERS.len() - 1),
            play_to: progress.last_play_to.clamp(1, 15),
            mana_mode: ManaMode::ALL[progress.last_mana_mode.min(2)],
            difficulty: Difficulty::ALL[progress.last_difficulty.min(2)],
            local2p: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChampionshipRun {
    pub round: usize,
    pub sport: Sport,
    pub arena: usize,
    pub player: usize,
    pub opponents: [usize; 3],
    pub difficulty: Difficulty,
    pub scores: Vec<[u32; 2]>,
    pub done: bool,
    pub won: bool,
}

pub struct FrontEnd {
    pub screen: Screen,
    pub setup: SetupState,
    pub championship_setup: bool,
    title_focus: usize,
    online_panel: OnlinePanel,
    online_focus: usize,
    join_addr: String,
    online_host: bool,
    lan: Option<LanBrowser>,
    pub notice: Option<String>,
    setup_focus: usize,
    roster_slot: usize,
    pub roster_index: usize,
    pub arena_index: usize,
    settings_focus: usize,
    controls_focus: usize,
    capture: Option<usize>,
    result_focus: usize,
    last_report: Option<MatchReport>,
    pub championship: Option<ChampionshipRun>,
    pending_action: Option<UiAction>,
    pub dirty: bool,
}

#[derive(Clone, Copy)]
struct Pointer {
    x: f32,
    y: f32,
    click: bool,
    down: bool,
}

impl Pointer {
    fn new(canvas: &Canvas) -> Self {
        let m = canvas.mouse();
        Self {
            x: m.x,
            y: m.y,
            click: is_mouse_button_pressed(MouseButton::Left),
            down: is_mouse_button_down(MouseButton::Left),
        }
    }

    fn over(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.x >= x && self.y >= y && self.x <= x + w && self.y <= y + h
    }

    fn hit(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.click && self.over(x, y, w, h)
    }
}

#[derive(Default, Clone, Copy)]
struct Nav {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    confirm: bool,
    back: bool,
}

impl Nav {
    fn read() -> Self {
        Self {
            up: is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W),
            down: is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S),
            left: is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A),
            right: is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D),
            confirm: is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space),
            back: is_key_pressed(KeyCode::Escape),
        }
    }
}

fn seed_now() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x5eed);
    nanos ^ (get_time().to_bits()).rotate_left(17)
}

fn sport_index(sport: Sport) -> usize {
    match sport {
        Sport::Soccer => 0,
        Sport::Volleyball => 1,
        Sport::Basketball => 2,
    }
}

fn mana_index(mode: ManaMode) -> usize {
    match mode {
        ManaMode::Bonus => 0,
        ManaMode::Offensive => 1,
        ManaMode::Round => 2,
    }
}

fn difficulty_index(difficulty: Difficulty) -> usize {
    match difficulty {
        Difficulty::Easy => 0,
        Difficulty::Normal => 1,
        Difficulty::Hard => 2,
    }
}

fn short_name(name: &str, max: usize) -> String {
    let upper = i18n::localize(&name.to_uppercase());
    if upper.chars().count() <= max {
        upper
    } else {
        let mut out: String = upper.chars().take(max.saturating_sub(1)).collect();
        out.push('.');
        out
    }
}

fn draw_slime(assets: &Assets, index: usize, center_x: f32, bottom: f32, width: f32, flip: bool) {
    let texture = assets.slime_for(index);
    let height = texture.height() * (width / texture.width());
    draw_texture_ex(
        texture,
        center_x - width / 2.0,
        bottom - height,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(width, height)),
            flip_x: flip,
            ..Default::default()
        },
    );
}

fn draw_slime_shadow(x: f32, y: f32, width: f32) {
    draw_ellipse(
        x,
        y,
        width * 0.5,
        width * 0.1,
        0.0,
        render::rgba([8, 12, 16], 0.3),
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_button(
    assets: &Assets,
    label: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    focused: bool,
    hovered: bool,
    accent: Color,
) {
    let active = focused || hovered;
    let fill = if active {
        render::rgba([24, 33, 48], 0.98)
    } else {
        render::rgba([12, 18, 29], 0.92)
    };
    let border = if active {
        accent
    } else {
        render::rgba([54, 67, 88], 0.95)
    };
    render::panel(x, y, w, h, fill, border);
    let scale = 0.36;
    let th = render::text_height(label, scale);
    let color = if active {
        accent
    } else {
        render::rgba([218, 228, 244], 1.0)
    };
    render::draw_text_px(
        assets,
        label,
        x + w / 2.0,
        y + (h - th) / 2.0,
        scale,
        color,
        TextAlign::Center,
        false,
    );
}

fn draw_header(assets: &Assets, title: &str, subtitle: &str) {
    render::draw_text_px(
        assets,
        title,
        24.0,
        10.0,
        0.62,
        render::rgba([255, 246, 214], 1.0),
        TextAlign::Left,
        true,
    );
    if !subtitle.is_empty() {
        render::draw_text_px(
            assets,
            subtitle,
            W - 100.0,
            15.0,
            0.28,
            render::rgba([172, 186, 206], 0.95),
            TextAlign::Right,
            false,
        );
    }
}

fn draw_back_button(assets: &Assets, pointer: &Pointer) -> (f32, f32, f32, f32) {
    let rect = (W - 86.0, 6.0, 76.0, 22.0);
    let hovered = pointer.over(rect.0, rect.1, rect.2, rect.3);
    draw_button(
        assets,
        "BACK",
        rect.0,
        rect.1,
        rect.2,
        rect.3,
        false,
        hovered,
        render::rgba([255, 212, 90], 1.0),
    );
    rect
}

#[allow(clippy::too_many_arguments)]
fn option_row(
    assets: &Assets,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    value: &str,
    focused: bool,
    hovered: bool,
    enabled: bool,
) -> (f32, f32, f32, f32) {
    let active = focused || hovered;
    let fill = if active {
        render::rgba([24, 33, 48], 0.98)
    } else {
        render::rgba([12, 18, 29], 0.88)
    };
    let border = if active {
        render::rgba([255, 212, 90], 1.0)
    } else {
        render::rgba([48, 60, 80], 0.9)
    };
    render::panel(x, y, w, h, fill, border);
    let label_color = if enabled {
        render::rgba([150, 164, 184], 1.0)
    } else {
        render::rgba([92, 102, 118], 1.0)
    };
    let value_color = if enabled {
        if active {
            render::rgba([255, 244, 214], 1.0)
        } else {
            render::rgba([222, 232, 246], 1.0)
        }
    } else {
        render::rgba([120, 130, 146], 1.0)
    };
    render::draw_text_px(
        assets,
        label,
        x + 8.0,
        y + 6.0,
        0.25,
        label_color,
        TextAlign::Left,
        false,
    );
    render::draw_text_px(
        assets,
        value,
        x + w - 8.0,
        y + 6.0,
        0.28,
        value_color,
        TextAlign::Right,
        false,
    );
    (x, y, w, h)
}

impl FrontEnd {
    pub fn new(save: &SaveData) -> Self {
        Self {
            screen: Screen::Title,
            setup: SetupState::from_progress(&save.progress),
            championship_setup: false,
            title_focus: 0,
            online_panel: OnlinePanel::Menu,
            online_focus: 0,
            join_addr: "127.0.0.1".into(),
            online_host: false,
            lan: None,
            notice: None,
            setup_focus: 0,
            roster_slot: 0,
            roster_index: save.progress.last_p1.min(CHARACTERS.len() - 1),
            arena_index: save.progress.last_arena.min(ARENAS.len() - 1),
            settings_focus: 0,
            controls_focus: 0,
            capture: None,
            result_focus: 0,
            last_report: None,
            championship: None,
            pending_action: None,
            dirty: false,
        }
    }

    pub fn to_title(&mut self) {
        self.screen = Screen::Title;
        self.title_focus = 0;
        self.capture = None;
        self.online_host = false;
        self.lan = None;
    }

    pub fn to_online(&mut self) {
        self.screen = Screen::Online;
        self.online_panel = OnlinePanel::Menu;
        self.online_focus = 0;
        self.online_host = false;
        self.lan = None;
        self.capture = None;
        self.championship = None;
    }

    pub fn to_online_notice(&mut self, notice: impl Into<String>) {
        self.to_online();
        self.notice = Some(notice.into());
    }

    pub fn on_match_finished(&mut self, report: MatchReport, save: &mut SaveData) {
        save.progress.matches += 1;
        let credit = if report.request.mode == LaunchMode::Online {
            report.request.local_side
        } else {
            0
        };
        if report.winner == credit {
            save.progress.wins += 1;
        }
        save.progress.goals += report.score[credit];
        save.progress.last_sport = sport_index(report.request.config.sport);
        save.progress.last_arena = report.request.config.arena;
        save.progress.last_p1 = report.request.config.p1;
        save.progress.last_p2 = report.request.config.p2;
        save.progress.last_play_to = report.request.config.play_to;
        save.progress.last_mana_mode = mana_index(report.request.config.mana_mode);
        save.progress.last_difficulty = difficulty_index(report.request.difficulty);

        if report.request.mode == LaunchMode::Championship {
            if let Some(run) = &mut self.championship {
                run.scores.push(report.score);
                if report.winner == 0 {
                    if run.round + 1 >= run.opponents.len() {
                        run.done = true;
                        run.won = true;
                        save.progress.championships += 1;
                        save.progress
                            .trophies
                            .push(format!("{} CUP", run.sport.label().to_uppercase()));
                    } else {
                        run.round += 1;
                    }
                } else {
                    run.done = true;
                    run.won = false;
                }
            }
        }

        self.last_report = Some(report);
        self.result_focus = 0;
        self.screen = Screen::Results;
        self.capture = None;
        self.dirty = true;
    }

    pub fn frame(
        &mut self,
        canvas: &mut Canvas,
        assets: &Assets,
        save: &mut SaveData,
        audio: &Audio,
    ) -> Option<UiAction> {
        self.pending_action = None;
        let time = get_time() as f32;
        let pointer = Pointer::new(canvas);
        let nav = Nav::read();

        canvas.begin_world(0.0, time);
        if self.screen == Screen::Results {
            let winner = self.last_report.as_ref().map(|r| r.winner).unwrap_or(0);
            render::draw_result_backdrop(assets, self.setup.arena, winner);
        } else {
            render::draw_menu_background(assets, self.setup.arena, time);
        }
        canvas.begin_ui();

        match self.screen {
            Screen::Title => self.screen_title(assets, save, audio, &pointer, &nav, time),
            Screen::Online => self.screen_online(assets, audio, &pointer, &nav, time),
            Screen::Setup => self.screen_setup(assets, save, audio, &pointer, &nav),
            Screen::Roster => self.screen_roster(assets, audio, &pointer, &nav),
            Screen::Arena => self.screen_arena(assets, audio, &pointer, &nav),
            Screen::Settings => self.screen_settings(assets, save, audio, &pointer, &nav),
            Screen::Controls => self.screen_controls(assets, save, audio, &pointer, &nav),
            Screen::Help => self.screen_help(assets, audio, &pointer, &nav),
            Screen::Results => self.screen_results(assets, save, audio, &pointer, &nav),
        }

        canvas.finish();
        self.pending_action.take()
    }

    fn screen_title(
        &mut self,
        assets: &Assets,
        save: &SaveData,
        audio: &Audio,
        pointer: &Pointer,
        nav: &Nav,
        time: f32,
    ) {
        let accent = render::rgba([255, 212, 90], 1.0);
        draw_slime_shadow(58.0, 384.0, 120.0);
        draw_slime(
            assets,
            save.progress.last_p1.min(CHARACTERS.len() - 1),
            58.0,
            384.0,
            124.0 * (1.0 + (time * 2.0).sin() * 0.015),
            false,
        );
        draw_slime_shadow(W - 58.0, 384.0, 120.0);
        draw_slime(
            assets,
            save.progress.last_p2.min(CHARACTERS.len() - 1),
            W - 58.0,
            384.0,
            124.0 * (1.0 + (time * 2.1 + 1.3).sin() * 0.015),
            true,
        );

        let ball = assets.ball_for(Sport::ALL[(time as usize / 4) % 3]);
        draw_texture_ex(
            ball,
            W / 2.0 + 118.0,
            30.0 + (time * 2.4).sin() * 5.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(21.0, 21.0)),
                rotation: time * 1.7,
                ..Default::default()
            },
        );

        render::draw_text_px(
            assets,
            "SUPER",
            W / 2.0,
            16.0,
            0.72,
            render::rgba([255, 222, 120], 1.0),
            TextAlign::Center,
            true,
        );
        render::draw_text_px(
            assets,
            "SLIME",
            W / 2.0,
            42.0,
            2.05,
            render::rgba([255, 250, 230], 1.0),
            TextAlign::Center,
            true,
        );
        render::draw_text_px(
            assets,
            "SOCCER",
            W / 2.0,
            118.0,
            1.2,
            render::rgba([142, 230, 79], 1.0),
            TextAlign::Center,
            true,
        );
        render::draw_text_px(
            assets,
            "SOCCER  /  VOLLEYBALL  /  BASKETBALL",
            W / 2.0,
            164.0,
            0.3,
            render::rgba([190, 204, 222], 0.95),
            TextAlign::Center,
            true,
        );

        let menu_x = 156.0;
        let menu_w = 248.0;
        let start_y = 188.0;
        for (i, label) in TITLE_ITEMS.iter().enumerate() {
            let y = start_y + i as f32 * 28.0;
            let hover = pointer.over(menu_x, y, menu_w, 26.0);
            draw_button(
                assets,
                label,
                menu_x,
                y,
                menu_w,
                26.0,
                self.title_focus == i,
                hover,
                accent,
            );
        }

        if nav.down {
            self.title_focus = (self.title_focus + 1) % TITLE_ITEMS.len();
            audio.play(Sfx::Ui);
        }
        if nav.up {
            self.title_focus = (self.title_focus + TITLE_ITEMS.len() - 1) % TITLE_ITEMS.len();
            audio.play(Sfx::Ui);
        }
        let clicked = (0..TITLE_ITEMS.len()).find(|i| {
            let y = start_y + *i as f32 * 28.0;
            pointer.hit(menu_x, y, menu_w, 26.0)
        });
        if nav.confirm || clicked.is_some() {
            if let Some(index) = clicked {
                self.title_focus = index;
            }
            self.activate_title(save, audio);
        }

        render::draw_text_px(
            assets,
            "OPEN SOURCE REMAKE  ·  RUST + MACROQUAD",
            W / 2.0,
            374.0,
            0.24,
            render::rgba([148, 160, 180], 0.9),
            TextAlign::Center,
            false,
        );
        if save.progress.championships > 0 {
            render::draw_text_px(
                assets,
                &format!("CUPS WON: {}", save.progress.championships),
                20.0,
                374.0,
                0.24,
                render::rgba([255, 218, 110], 0.9),
                TextAlign::Left,
                false,
            );
        }
    }

    fn activate_title(&mut self, save: &SaveData, audio: &Audio) {
        audio.play(Sfx::Ui);
        match self.title_focus {
            0 => {
                self.setup = SetupState::from_progress(&save.progress);
                self.setup.local2p = false;
                self.championship_setup = false;
                self.online_host = false;
                self.screen = Screen::Setup;
            }
            1 => {
                self.setup = SetupState::from_progress(&save.progress);
                self.setup.local2p = false;
                self.championship_setup = true;
                self.online_host = false;
                self.screen = Screen::Setup;
            }
            2 => {
                self.setup = SetupState::from_progress(&save.progress);
                self.setup.local2p = false;
                self.championship_setup = false;
                self.online_host = false;
                self.online_panel = OnlinePanel::Menu;
                self.online_focus = 0;
                self.lan = None;
                self.notice = None;
                self.screen = Screen::Online;
            }
            3 => self.screen = Screen::Settings,
            4 => self.screen = Screen::Help,
            5 => self.pending_action = Some(UiAction::Quit),
            _ => {}
        }
    }

    fn screen_online(
        &mut self,
        assets: &Assets,
        audio: &Audio,
        pointer: &Pointer,
        nav: &Nav,
        time: f32,
    ) {
        match self.online_panel {
            OnlinePanel::Menu => self.online_menu(assets, audio, pointer, nav),
            OnlinePanel::Join => self.online_join(assets, audio, pointer, nav, time),
            OnlinePanel::Lan => self.online_lan(assets, audio, pointer, nav),
        }
    }

    fn online_menu(&mut self, assets: &Assets, audio: &Audio, pointer: &Pointer, nav: &Nav) {
        draw_header(assets, "ONLINE PLAY", "LAN + INTERNET");
        render::panel(
            44.0,
            62.0,
            W - 88.0,
            48.0,
            render::rgba([10, 16, 26], 0.88),
            render::rgba([44, 56, 74], 0.9),
        );
        render::draw_text_px(
            assets,
            "THE HOST RUNS THE MATCH. EVERYONE ELSE SENDS INPUT.",
            W / 2.0,
            70.0,
            0.25,
            render::rgba([198, 211, 228], 0.95),
            TextAlign::Center,
            false,
        );
        render::draw_text_px(
            assets,
            "UDP  ·  120 HZ SIM  ·  30 HZ SNAPSHOTS  ·  PORT 43210",
            W / 2.0,
            90.0,
            0.21,
            render::rgba([142, 158, 180], 0.95),
            TextAlign::Center,
            false,
        );

        let items = ["HOST GAME", "JOIN BY IP", "LAN GAMES"];
        let x = 150.0;
        let w = 260.0;
        let h = 36.0;
        let start_y = 136.0;
        for (index, label) in items.iter().enumerate() {
            let y = start_y + index as f32 * 46.0;
            let hover = pointer.over(x, y, w, h);
            draw_button(
                assets,
                label,
                x,
                y,
                w,
                h,
                self.online_focus == index,
                hover,
                render::rgba([142, 230, 79], 1.0),
            );
            if hover {
                self.online_focus = index;
            }
        }

        if nav.down {
            self.online_focus = (self.online_focus + 1) % items.len();
            audio.play(Sfx::Ui);
        }
        if nav.up {
            self.online_focus = (self.online_focus + items.len() - 1) % items.len();
            audio.play(Sfx::Ui);
        }
        let clicked = (0..items.len()).find(|index| {
            let y = start_y + *index as f32 * 46.0;
            pointer.hit(x, y, w, h)
        });
        if nav.confirm || clicked.is_some() {
            if let Some(index) = clicked {
                self.online_focus = index;
            }
            self.activate_online_menu(audio);
        }

        if let Some(notice) = &self.notice {
            render::draw_text_px(
                assets,
                &render::wrap_text(assets, notice, 58, 0.24),
                W / 2.0,
                286.0,
                0.24,
                render::rgba([255, 154, 142], 1.0),
                TextAlign::Center,
                false,
            );
        } else {
            render::draw_text_px(
                assets,
                "HOST A MATCH OR FIND ONE ON YOUR NETWORK.",
                W / 2.0,
                294.0,
                0.24,
                render::rgba([150, 166, 188], 0.95),
                TextAlign::Center,
                false,
            );
        }

        let back = draw_back_button(assets, pointer);
        if nav.back || pointer.hit(back.0, back.1, back.2, back.3) {
            self.to_title();
            audio.play(Sfx::Ui);
        }
    }

    fn activate_online_menu(&mut self, audio: &Audio) {
        audio.play(Sfx::Ui);
        self.notice = None;
        match self.online_focus {
            0 => {
                self.championship_setup = false;
                self.championship = None;
                self.online_host = true;
                self.setup.local2p = false;
                self.screen = Screen::Setup;
            }
            1 => {
                self.online_panel = OnlinePanel::Join;
                self.online_focus = 0;
            }
            2 => {
                self.online_panel = OnlinePanel::Lan;
                self.online_focus = 0;
                if self.lan.is_none() {
                    match LanBrowser::new() {
                        Ok(browser) => self.lan = Some(browser),
                        Err(error) => {
                            self.notice = Some(format!("LAN UNAVAILABLE: {error}"));
                            self.online_panel = OnlinePanel::Menu;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn online_join(
        &mut self,
        assets: &Assets,
        audio: &Audio,
        pointer: &Pointer,
        nav: &Nav,
        time: f32,
    ) {
        draw_header(assets, "JOIN GAME", "TYPE THE HOST ADDRESS");

        render::draw_text_px(
            assets,
            "HOST ADDRESS",
            62.0,
            66.0,
            0.22,
            render::rgba([150, 166, 188], 0.95),
            TextAlign::Left,
            false,
        );
        let field = (60.0, 80.0, W - 120.0, 38.0);
        render::panel(
            field.0,
            field.1,
            field.2,
            field.3,
            render::rgba([14, 21, 33], 0.98),
            render::rgba([255, 212, 90], 0.9),
        );
        while let Some(ch) = get_char_pressed() {
            if self.join_addr.chars().count() < 32
                && (ch.is_ascii_alphanumeric() || matches!(ch, '.' | ':' | '-' | '_'))
            {
                self.join_addr.push(ch);
                self.notice = None;
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            self.join_addr.pop();
            self.notice = None;
        }
        let caret = if (time * 2.0) as i32 % 2 == 0 {
            "_"
        } else {
            ""
        };
        let shown = if self.join_addr.is_empty() {
            format!("192.168.1.20:43210{caret}")
        } else {
            format!("{}{caret}", self.join_addr)
        };
        render::draw_text_px(
            assets,
            &shown,
            field.0 + 12.0,
            field.1 + 8.0,
            0.38,
            if self.join_addr.is_empty() {
                render::rgba([108, 121, 140], 0.9)
            } else {
                render::rgba([245, 249, 255], 1.0)
            },
            TextAlign::Left,
            false,
        );

        let card = (60.0, 134.0, W - 120.0, 116.0);
        render::panel(
            card.0,
            card.1,
            card.2,
            card.3,
            render::rgba([12, 18, 29], 0.94),
            render::rgba([48, 60, 80], 0.9),
        );
        render::draw_text_px(
            assets,
            "YOUR SLIME",
            card.0 + 10.0,
            card.1 + 6.0,
            0.22,
            render::rgba([150, 166, 188], 0.95),
            TextAlign::Left,
            false,
        );
        let character = &CHARACTERS[self.setup.p1];
        draw_slime_shadow(card.0 + 60.0, card.1 + 96.0, 78.0);
        draw_slime(
            assets,
            self.setup.p1,
            card.0 + 60.0,
            card.1 + 96.0,
            94.0,
            false,
        );
        render::draw_text_px(
            assets,
            &short_name(character.name, 18),
            card.0 + 118.0,
            card.1 + 30.0,
            0.42,
            render::rgba([245, 248, 255], 1.0),
            TextAlign::Left,
            false,
        );
        render::draw_text_px(
            assets,
            &character.ability.name.to_uppercase(),
            card.0 + 118.0,
            card.1 + 54.0,
            0.25,
            render::rgba([255, 212, 90], 0.95),
            TextAlign::Left,
            false,
        );
        render::draw_text_px(
            assets,
            &format!(
                "SPD {:.2}  JMP {:.2}  PWR {:.2}",
                character.stats.speed, character.stats.jump, character.stats.power
            ),
            card.0 + 118.0,
            card.1 + 74.0,
            0.21,
            render::rgba([166, 180, 200], 0.95),
            TextAlign::Left,
            false,
        );
        render::draw_text_px(
            assets,
            "ARROW KEYS CHANGE SLIME",
            card.0 + 118.0,
            card.1 + 92.0,
            0.19,
            render::rgba([130, 145, 165], 0.9),
            TextAlign::Left,
            false,
        );

        let prev = (card.0 + card.2 - 72.0, card.1 + 42.0, 28.0, 28.0);
        let next = (card.0 + card.2 - 36.0, card.1 + 42.0, 28.0, 28.0);
        for (rect, label) in [(prev, "<"), (next, ">")] {
            let hover = pointer.over(rect.0, rect.1, rect.2, rect.3);
            draw_button(
                assets,
                label,
                rect.0,
                rect.1,
                rect.2,
                rect.3,
                false,
                hover,
                render::rgba([255, 212, 90], 1.0),
            );
        }
        let mut shift = 0i32;
        if pointer.hit(prev.0, prev.1, prev.2, prev.3) || is_key_pressed(KeyCode::Left) {
            shift = -1;
        }
        if pointer.hit(next.0, next.1, next.2, next.3) || is_key_pressed(KeyCode::Right) {
            shift = 1;
        }
        if shift != 0 {
            self.setup.p1 =
                (self.setup.p1 as i32 + shift).rem_euclid(CHARACTERS.len() as i32) as usize;
            audio.play(Sfx::Ui);
        }

        let connect = (W / 2.0 - 118.0, 268.0, 236.0, 38.0);
        let hover = pointer.over(connect.0, connect.1, connect.2, connect.3);
        draw_button(
            assets,
            "CONNECT",
            connect.0,
            connect.1,
            connect.2,
            connect.3,
            true,
            hover,
            render::rgba([142, 230, 79], 1.0),
        );
        let connect_now = pointer.hit(connect.0, connect.1, connect.2, connect.3) || nav.confirm;
        if connect_now {
            match normalize_address(&self.join_addr) {
                Ok(_) => {
                    self.pending_action = Some(UiAction::JoinOnline {
                        addr: self.join_addr.clone(),
                        char_id: self.setup.p1,
                    });
                    audio.play(Sfx::Go);
                }
                Err(error) => {
                    self.notice = Some(error.to_uppercase());
                    audio.play(Sfx::Deny);
                }
            }
        }

        if let Some(notice) = &self.notice {
            render::draw_text_px(
                assets,
                &render::wrap_text(assets, notice, 58, 0.23),
                W / 2.0,
                318.0,
                0.23,
                render::rgba([255, 154, 142], 1.0),
                TextAlign::Center,
                false,
            );
        }
        render::draw_text_px(
            assets,
            "ENTER CONNECT  ·  ESC BACK",
            W / 2.0,
            356.0,
            0.22,
            render::rgba([128, 142, 162], 0.9),
            TextAlign::Center,
            false,
        );

        let back = draw_back_button(assets, pointer);
        if nav.back || pointer.hit(back.0, back.1, back.2, back.3) {
            self.online_panel = OnlinePanel::Menu;
            self.online_focus = 0;
            self.notice = None;
            audio.play(Sfx::Ui);
        }
    }

    fn online_lan(&mut self, assets: &Assets, audio: &Audio, pointer: &Pointer, nav: &Nav) {
        if let Some(browser) = self.lan.as_mut() {
            browser.update();
        }
        let offers: Vec<LanOffer> = self
            .lan
            .as_ref()
            .map(|browser| browser.offers().to_vec())
            .unwrap_or_default();

        draw_header(assets, "LAN GAMES", "AUTO-DISCOVERY");
        if offers.is_empty() {
            render::panel(
                52.0,
                82.0,
                W - 104.0,
                128.0,
                render::rgba([10, 16, 26], 0.86),
                render::rgba([44, 56, 74], 0.9),
            );
            render::draw_text_px(
                assets,
                "SEARCHING THE LOCAL NETWORK...",
                W / 2.0,
                112.0,
                0.32,
                render::rgba([214, 226, 242], 0.95),
                TextAlign::Center,
                false,
            );
            render::draw_text_px(
                assets,
                "MAKE SURE THE HOST HAS A MATCH OPEN.",
                W / 2.0,
                144.0,
                0.24,
                render::rgba([158, 174, 196], 0.95),
                TextAlign::Center,
                false,
            );
            render::draw_text_px(
                assets,
                "SOME FIREWALLS BLOCK LAN DISCOVERY. JOIN BY IP INSTEAD.",
                W / 2.0,
                166.0,
                0.2,
                render::rgba([130, 145, 165], 0.9),
                TextAlign::Center,
                false,
            );
        } else {
            let visible = offers.len().min(4);
            if self.online_focus >= visible {
                self.online_focus = visible - 1;
            }
            for (index, offer) in offers.iter().take(visible).enumerate() {
                let x = 40.0;
                let y = 64.0 + index as f32 * 46.0;
                let w = W - 80.0;
                let h = 40.0;
                let hover = pointer.over(x, y, w, h);
                if hover {
                    self.online_focus = index;
                }
                let focused = self.online_focus == index;
                let open = !offer.in_match && offer.open_slots() > 0;
                render::panel(
                    x,
                    y,
                    w,
                    h,
                    if focused {
                        render::rgba([24, 35, 50], 0.98)
                    } else {
                        render::rgba([10, 16, 26], 0.9)
                    },
                    if focused {
                        render::rgba([255, 212, 90], 1.0)
                    } else {
                        render::rgba([48, 60, 80], 0.9)
                    },
                );
                render::draw_text_px(
                    assets,
                    &short_name(&offer.name, 20),
                    x + 12.0,
                    y + 5.0,
                    0.33,
                    if focused {
                        render::rgba([255, 246, 214], 1.0)
                    } else {
                        render::rgba([230, 238, 250], 1.0)
                    },
                    TextAlign::Left,
                    false,
                );
                render::draw_text_px(
                    assets,
                    &format!(
                        "{}  ·  {} / {} PLAYERS",
                        offer.sport.label().to_uppercase(),
                        offer.players,
                        offer.max_players
                    ),
                    x + 12.0,
                    y + 19.0,
                    0.22,
                    render::rgba([158, 174, 196], 0.95),
                    TextAlign::Left,
                    false,
                );
                render::draw_text_px(
                    assets,
                    if offer.in_match {
                        "IN MATCH"
                    } else if open {
                        "OPEN"
                    } else {
                        "FULL"
                    },
                    x + w - 12.0,
                    y + 10.0,
                    0.27,
                    if open {
                        render::rgba([142, 230, 79], 1.0)
                    } else {
                        render::rgba([255, 140, 132], 0.95)
                    },
                    TextAlign::Right,
                    false,
                );
                if pointer.hit(x, y, w, h) {
                    self.online_focus = index;
                    self.join_lan_offer(offer, audio);
                    return;
                }
            }
            if offers.len() > 4 {
                render::draw_text_px(
                    assets,
                    &format!("+{} MORE", offers.len() - 4),
                    W / 2.0,
                    258.0,
                    0.22,
                    render::rgba([142, 158, 180], 0.9),
                    TextAlign::Center,
                    false,
                );
            }
            if nav.down {
                self.online_focus = (self.online_focus + 1) % visible;
                audio.play(Sfx::Ui);
            }
            if nav.up {
                self.online_focus = (self.online_focus + visible - 1) % visible;
                audio.play(Sfx::Ui);
            }
            if nav.confirm {
                if let Some(offer) = offers.get(self.online_focus) {
                    self.join_lan_offer(offer, audio);
                }
            }
        }

        let ip_button = (40.0, 282.0, 150.0, 32.0);
        let hover = pointer.over(ip_button.0, ip_button.1, ip_button.2, ip_button.3);
        draw_button(
            assets,
            "JOIN BY IP",
            ip_button.0,
            ip_button.1,
            ip_button.2,
            ip_button.3,
            false,
            hover,
            render::rgba([255, 212, 90], 1.0),
        );
        if pointer.hit(ip_button.0, ip_button.1, ip_button.2, ip_button.3) {
            self.online_panel = OnlinePanel::Join;
            self.online_focus = 0;
            self.notice = None;
            audio.play(Sfx::Ui);
        }
        if let Some(notice) = &self.notice {
            render::draw_text_px(
                assets,
                &render::wrap_text(assets, notice, 50, 0.22),
                W - 40.0,
                292.0,
                0.22,
                render::rgba([255, 154, 142], 1.0),
                TextAlign::Right,
                false,
            );
        }

        let back = draw_back_button(assets, pointer);
        if nav.back || pointer.hit(back.0, back.1, back.2, back.3) {
            self.online_panel = OnlinePanel::Menu;
            self.online_focus = 0;
            self.notice = None;
            audio.play(Sfx::Ui);
        }
    }

    fn join_lan_offer(&mut self, offer: &LanOffer, audio: &Audio) {
        if offer.in_match || offer.open_slots() == 0 {
            self.notice = Some("THAT MATCH IS ALREADY FULL.".into());
            audio.play(Sfx::Deny);
            return;
        }
        self.pending_action = Some(UiAction::JoinOnline {
            addr: offer.addr.to_string(),
            char_id: self.setup.p1,
        });
        audio.play(Sfx::Go);
    }

    fn screen_setup(
        &mut self,
        assets: &Assets,
        save: &mut SaveData,
        audio: &Audio,
        pointer: &Pointer,
        nav: &Nav,
    ) {
        let title = if self.championship_setup {
            "CHAMPIONSHIP SETUP"
        } else if self.online_host {
            "HOST ONLINE MATCH"
        } else {
            "MATCH SETUP"
        };
        draw_header(
            assets,
            title,
            if self.championship_setup {
                "THREE WINS. ONE CUP."
            } else if self.online_host {
                "PLAYER 2 JOINS OVER THE NETWORK"
            } else {
                "PICK YOUR SLIME AND RULES"
            },
        );

        let card_y = 42.0;
        let card_h = 108.0;
        let p1_rect = (18.0, card_y, 202.0, card_h);
        let p2_rect = (W - 220.0, card_y, 202.0, card_h);

        self.draw_player_card(
            assets,
            0,
            p1_rect,
            self.setup_focus == 0,
            pointer.over(p1_rect.0, p1_rect.1, p1_rect.2, p1_rect.3),
            pointer,
            "PLAYER 1",
        );
        let p2_label = if self.championship_setup {
            "RANDOM RIVALS"
        } else if self.online_host {
            "REMOTE RIVAL"
        } else if self.setup.local2p {
            "PLAYER 2"
        } else {
            "CPU RIVAL"
        };
        self.draw_player_card(
            assets,
            1,
            p2_rect,
            self.setup_focus == 1,
            pointer.over(p2_rect.0, p2_rect.1, p2_rect.2, p2_rect.3),
            pointer,
            p2_label,
        );

        let vs_w = 44.0;
        let vs_h = 55.0;
        draw_texture_ex(
            &assets.vs,
            W / 2.0 - vs_w / 2.0,
            card_y + 22.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(vs_w, vs_h)),
                ..Default::default()
            },
        );

        let row_x = 18.0;
        let row_w = 254.0;
        let row_h = 27.0;
        let left_y = 160.0;
        let right_x = W - 272.0;

        let sport_value = self.setup.sport.label().to_uppercase();
        let arena_value = short_name(ARENAS[self.setup.arena].name, 15);
        let mana_value = self.setup.mana_mode.label().to_uppercase();
        let play_value = format!("{} GOALS", self.setup.play_to);
        let diff_value = self.setup.difficulty.label().to_uppercase();
        let opp_value = if self.championship_setup {
            "CPU BRACKET".to_string()
        } else if self.online_host {
            "ONLINE".to_string()
        } else if self.setup.local2p {
            "LOCAL 2P".to_string()
        } else {
            "CPU".to_string()
        };

        let rows = [
            ("SPORT", sport_value, true),
            ("ARENA", arena_value, true),
            ("MANA MODE", mana_value, true),
            ("PLAY TO", play_value, true),
            ("DIFFICULTY", diff_value, !self.online_host),
            (
                "OPPONENT",
                opp_value,
                !self.championship_setup && !self.online_host,
            ),
            (
                "FORMAT",
                if self.championship_setup {
                    "3 ROUND CUP"
                } else if self.online_host {
                    "ONLINE MATCH"
                } else {
                    "SINGLE MATCH"
                }
                .to_string(),
                false,
            ),
        ];
        let positions = [
            (row_x, left_y),
            (row_x, left_y + 31.0),
            (row_x, left_y + 62.0),
            (row_x, left_y + 93.0),
            (right_x, left_y),
            (right_x, left_y + 31.0),
            (right_x, left_y + 62.0),
        ];
        for (i, (label, value, enabled)) in rows.iter().enumerate() {
            let (x, y) = positions[i];
            let hovered = pointer.over(x, y, row_w, row_h);
            option_row(
                assets,
                x,
                y,
                row_w,
                row_h,
                label,
                value,
                i < 6 && self.setup_focus == i + 2,
                hovered,
                *enabled,
            );
            if i < 6 && pointer.over(x, y, row_w, row_h) && pointer.click {
                if self.online_host && i >= 4 {
                    continue;
                }
                self.setup_focus = i + 2;
                match i {
                    0 => self.cycle_sport(1),
                    1 => self.cycle_arena(1),
                    2 => self.cycle_mana(1),
                    3 => self.adjust_play_to(1),
                    4 => self.cycle_difficulty(1),
                    5 if !self.championship_setup => self.setup.local2p = !self.setup.local2p,
                    _ => {}
                }
                audio.play(Sfx::Ui);
            }
        }

        if nav.left || nav.right {
            let dir = if nav.right { 1 } else { -1 };
            match self.setup_focus {
                2 => self.cycle_sport(dir),
                3 => self.cycle_arena(dir),
                4 => self.cycle_mana(dir),
                5 => self.adjust_play_to(dir),
                6 if !self.online_host => self.cycle_difficulty(dir),
                7 if !self.championship_setup && !self.online_host => {
                    self.setup.local2p = !self.setup.local2p
                }
                _ => {}
            }
            if (2..=7).contains(&self.setup_focus) && !(self.online_host && self.setup_focus >= 6) {
                audio.play(Sfx::Ui);
            }
        }
        if nav.down {
            self.setup_focus = self.next_setup_focus(1);
            audio.play(Sfx::Ui);
        }
        if nav.up {
            self.setup_focus = self.next_setup_focus(-1);
            audio.play(Sfx::Ui);
        }

        let p1_click = pointer.hit(p1_rect.0, p1_rect.1, p1_rect.2, p1_rect.3);
        if p1_click {
            self.setup_focus = 0;
            self.open_roster(0, audio);
            return;
        }
        let p2_click = pointer.hit(p2_rect.0, p2_rect.1, p2_rect.2, p2_rect.3);
        if p2_click && !self.championship_setup && !self.online_host {
            self.setup_focus = 1;
            self.open_roster(1, audio);
            return;
        }

        let back = (18.0, 336.0, 104.0, 32.0);
        let start = (W / 2.0 - 118.0, 334.0, 236.0, 36.0);
        let back_hover = pointer.over(back.0, back.1, back.2, back.3);
        let start_hover = pointer.over(start.0, start.1, start.2, start.3);
        draw_button(
            assets,
            "CHANGE",
            back.0,
            back.1,
            back.2,
            back.3,
            self.setup_focus == 9,
            back_hover,
            render::rgba([170, 184, 204], 1.0),
        );
        draw_button(
            assets,
            if self.championship_setup {
                "START CUP"
            } else if self.online_host {
                "HOST MATCH"
            } else {
                "START MATCH"
            },
            start.0,
            start.1,
            start.2,
            start.3,
            self.setup_focus == 8,
            start_hover,
            render::rgba([142, 230, 79], 1.0),
        );

        if pointer.hit(back.0, back.1, back.2, back.3) {
            self.screen = Screen::Title;
            audio.play(Sfx::Ui);
        }
        if pointer.hit(start.0, start.1, start.2, start.3) {
            self.begin_setup_match(save, audio);
        }

        if nav.confirm {
            match self.setup_focus {
                0 => self.open_roster(0, audio),
                1 if !self.championship_setup && !self.online_host => self.open_roster(1, audio),
                2 => self.cycle_sport(1),
                3 => self.cycle_arena(1),
                4 => self.cycle_mana(1),
                5 => self.adjust_play_to(1),
                6 if !self.online_host => self.cycle_difficulty(1),
                7 if !self.championship_setup && !self.online_host => {
                    self.setup.local2p = !self.setup.local2p
                }
                8 => self.begin_setup_match(save, audio),
                9 => self.screen = Screen::Title,
                _ => {}
            }
            if self.setup_focus != 8
                && !nav_confirm_opens_roster(
                    self.setup_focus,
                    self.championship_setup,
                    self.online_host,
                )
            {
                audio.play(Sfx::Ui);
            }
        }
        if nav.back {
            self.screen = Screen::Title;
            audio.play(Sfx::Ui);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_player_card(
        &self,
        assets: &Assets,
        player: usize,
        rect: (f32, f32, f32, f32),
        focused: bool,
        hovered: bool,
        pointer: &Pointer,
        label: &str,
    ) {
        let (x, y, w, h) = rect;
        let active = focused || hovered;
        let border = if active {
            render::side_color(player)
        } else {
            render::rgba([54, 67, 88], 1.0)
        };
        let fill = if active {
            render::rgba([22, 30, 44], 0.98)
        } else {
            render::rgba([12, 18, 29], 0.92)
        };
        render::panel(x, y, w, h, fill, border);
        render::draw_text_px(
            assets,
            label,
            x + 8.0,
            y + 5.0,
            0.23,
            render::side_color(player),
            TextAlign::Left,
            false,
        );
        if self.championship_setup && player == 1 {
            render::draw_text_px(
                assets,
                "DRAWN",
                x + w - 8.0,
                y + 5.0,
                0.22,
                render::rgba([255, 212, 90], 0.9),
                TextAlign::Right,
                false,
            );
            for i in 0..3 {
                let idx = self.champ_preview_char(i) % CHARACTERS.len();
                draw_slime_shadow(x + 46.0 + i as f32 * 55.0, y + 74.0, 34.0);
                draw_slime(
                    assets,
                    idx,
                    x + 46.0 + i as f32 * 55.0,
                    y + 74.0,
                    46.0,
                    i % 2 == 1,
                );
            }
            render::draw_text_px(
                assets,
                "BEAT ALL THREE",
                x + w / 2.0,
                y + 84.0,
                0.25,
                render::rgba([190, 204, 222], 0.95),
                TextAlign::Center,
                false,
            );
            return;
        }
        let index = if player == 0 {
            self.setup.p1
        } else {
            self.setup.p2
        };
        let character = &CHARACTERS[index];
        draw_slime_shadow(x + 52.0, y + 76.0, 70.0);
        draw_slime(assets, index, x + 52.0, y + 76.0, 84.0, player == 1);
        render::draw_text_px(
            assets,
            &short_name(character.name, 18),
            x + 94.0,
            y + 35.0,
            0.34,
            render::rgba([245, 248, 255], 1.0),
            TextAlign::Left,
            false,
        );
        render::draw_text_px(
            assets,
            &character.ability.name.to_uppercase(),
            x + 94.0,
            y + 53.0,
            0.24,
            render::rgba([255, 212, 90], 0.95),
            TextAlign::Left,
            false,
        );
        render::draw_text_px(
            assets,
            &format!(
                "SPD {:.2}  JMP {:.2}  PWR {:.2}",
                character.stats.speed, character.stats.jump, character.stats.power
            ),
            x + 94.0,
            y + 69.0,
            0.2,
            render::rgba([166, 180, 200], 0.95),
            TextAlign::Left,
            false,
        );
        render::draw_text_px(
            assets,
            if self.online_host && player == 1 {
                "JOINER PICKS THEIR OWN SLIME"
            } else if pointer.over(x, y, w, h) {
                "CLICK TO CHANGE"
            } else {
                "SLIME READY"
            },
            x + 94.0,
            y + 86.0,
            0.2,
            render::rgba([139, 154, 174], 0.9),
            TextAlign::Left,
            false,
        );
    }

    fn champ_preview_char(&self, i: usize) -> usize {
        1 + (self.setup.p1 + 7 + i * 11) % (CHARACTERS.len() - 1)
    }

    fn open_roster(&mut self, slot: usize, audio: &Audio) {
        audio.play(Sfx::Ui);
        self.roster_slot = slot;
        self.roster_index = if slot == 0 {
            self.setup.p1
        } else {
            self.setup.p2
        };
        self.screen = Screen::Roster;
    }

    fn next_setup_focus(&self, dir: isize) -> usize {
        let mut focus = self.setup_focus as isize;
        for _ in 0..10 {
            focus = (focus + dir).rem_euclid(10);
            if self.online_host && (focus == 6 || focus == 7) {
                continue;
            }
            break;
        }
        focus as usize
    }

    fn cycle_sport(&mut self, dir: i32) {
        let index = sport_index(self.setup.sport);
        let next = (index as i32 + dir).rem_euclid(3) as usize;
        self.setup.sport = Sport::ALL[next];
    }

    fn cycle_arena(&mut self, dir: i32) {
        self.setup.arena = (self.setup.arena as i32 + dir).rem_euclid(ARENAS.len() as i32) as usize;
    }

    fn cycle_mana(&mut self, dir: i32) {
        let index = mana_index(self.setup.mana_mode);
        let next = (index as i32 + dir).rem_euclid(3) as usize;
        self.setup.mana_mode = ManaMode::ALL[next];
    }

    fn cycle_difficulty(&mut self, dir: i32) {
        let index = difficulty_index(self.setup.difficulty);
        let next = (index as i32 + dir).rem_euclid(3) as usize;
        self.setup.difficulty = Difficulty::ALL[next];
    }

    fn adjust_play_to(&mut self, dir: i32) {
        self.setup.play_to = (self.setup.play_to as i32 + dir).clamp(1, 15) as u32;
    }

    fn begin_setup_match(&mut self, save: &mut SaveData, audio: &Audio) {
        if self.championship_setup {
            self.start_championship(save, audio);
        } else {
            let seed = seed_now();
            let request = LaunchRequest {
                config: MatchConfig {
                    sport: self.setup.sport,
                    arena: self.setup.arena,
                    p1: self.setup.p1,
                    p2: self.setup.p2,
                    play_to: self.setup.play_to,
                    mana_mode: self.setup.mana_mode,
                    seed,
                    p1_control: 1.0,
                    p2_control: 1.0,
                },
                ai: !self.setup.local2p && !self.online_host,
                difficulty: self.setup.difficulty,
                mode: if self.online_host {
                    LaunchMode::Online
                } else {
                    LaunchMode::Casual
                },
                local_side: 0,
            };
            self.pending_action = Some(if self.online_host {
                UiAction::HostOnline(request)
            } else {
                UiAction::Launch(request)
            });
            audio.play(Sfx::Go);
        }
    }

    fn start_championship(&mut self, save: &mut SaveData, audio: &Audio) {
        let seed = seed_now();
        let mut rng = Rng::new(seed);
        let mut opponents = [0usize; 3];
        for slot in 0..3 {
            let mut candidate = (rng.next_u32() as usize) % CHARACTERS.len();
            let mut guard = 0;
            while candidate == self.setup.p1 || opponents[..slot].contains(&candidate) {
                candidate = (rng.next_u32() as usize) % CHARACTERS.len();
                guard += 1;
                if guard > 64 {
                    candidate = (self.setup.p1 + slot + 1) % CHARACTERS.len();
                }
            }
            opponents[slot] = candidate;
        }
        self.championship = Some(ChampionshipRun {
            round: 0,
            sport: self.setup.sport,
            arena: self.setup.arena,
            player: self.setup.p1,
            opponents,
            difficulty: self.setup.difficulty,
            scores: Vec::new(),
            done: false,
            won: false,
        });
        save.progress.last_p1 = self.setup.p1;
        self.dirty = true;
        audio.play(Sfx::Go);
        self.launch_championship_round();
    }

    fn launch_championship_round(&mut self) {
        let Some(run) = self.championship.clone() else {
            return;
        };
        if run.done || run.round >= run.opponents.len() {
            return;
        }
        let request = LaunchRequest {
            config: MatchConfig {
                sport: run.sport,
                arena: run.arena,
                p1: run.player,
                p2: run.opponents[run.round],
                play_to: self.setup.play_to,
                mana_mode: self.setup.mana_mode,
                seed: seed_now(),
                p1_control: 1.0,
                p2_control: 1.0,
            },
            ai: true,
            difficulty: run.difficulty,
            mode: LaunchMode::Championship,
            local_side: 0,
        };
        self.pending_action = Some(UiAction::Launch(request));
    }

    fn screen_roster(&mut self, assets: &Assets, audio: &Audio, pointer: &Pointer, nav: &Nav) {
        let slot = self.roster_slot;
        draw_header(
            assets,
            "CHOOSE SLIME",
            if slot == 0 {
                "PLAYER 1"
            } else {
                "PLAYER 2 / CPU"
            },
        );
        let cols = 7;
        let cell_w = 72.0;
        let cell_h = 54.0;
        let start_x = 28.0;
        let start_y = 48.0;

        if nav.left {
            self.roster_index = (self.roster_index + CHARACTERS.len() - 1) % CHARACTERS.len();
            audio.play(Sfx::Ui);
        }
        if nav.right {
            self.roster_index = (self.roster_index + 1) % CHARACTERS.len();
            audio.play(Sfx::Ui);
        }
        if nav.up {
            self.roster_index = (self.roster_index + CHARACTERS.len() - cols) % CHARACTERS.len();
            audio.play(Sfx::Ui);
        }
        if nav.down {
            self.roster_index = (self.roster_index + cols) % CHARACTERS.len();
            audio.play(Sfx::Ui);
        }

        for (index, character) in CHARACTERS.iter().enumerate() {
            let col = index % cols;
            let row = index / cols;
            let x = start_x + col as f32 * cell_w;
            let y = start_y + row as f32 * cell_h;
            let hovered = pointer.over(x + 1.0, y + 1.0, cell_w - 2.0, cell_h - 2.0);
            if hovered {
                self.roster_index = index;
            }
            let selected = self.roster_index == index;
            let chosen = if slot == 0 {
                self.setup.p1
            } else {
                self.setup.p2
            } == index;
            let fill = if selected {
                render::rgba([24, 35, 50], 0.98)
            } else if hovered {
                render::rgba([20, 29, 42], 0.96)
            } else {
                render::rgba([10, 16, 26], 0.86)
            };
            render::panel(
                x + 1.0,
                y + 1.0,
                cell_w - 2.0,
                cell_h - 2.0,
                fill,
                if selected {
                    render::rgba([255, 212, 90], 1.0)
                } else if chosen {
                    render::side_color(slot)
                } else {
                    render::rgba([44, 55, 72], 0.8)
                },
            );
            draw_slime_shadow(x + cell_w / 2.0, y + 42.0, 34.0);
            draw_slime(assets, index, x + cell_w / 2.0, y + 42.0, 45.0, slot == 1);
            render::draw_text_px(
                assets,
                &short_name(character.name, 11),
                x + cell_w / 2.0,
                y + 42.0,
                0.2,
                if selected {
                    render::rgba([255, 244, 214], 1.0)
                } else {
                    render::rgba([160, 174, 194], 0.95)
                },
                TextAlign::Center,
                false,
            );
            if pointer.hit(x + 1.0, y + 1.0, cell_w - 2.0, cell_h - 2.0) {
                self.roster_index = index;
                self.choose_roster(audio);
                return;
            }
        }

        if nav.confirm {
            self.choose_roster(audio);
            return;
        }
        let back = draw_back_button(assets, pointer);
        if nav.back || pointer.hit(back.0, back.1, back.2, back.3) {
            self.screen = Screen::Setup;
            self.setup_focus = slot;
            audio.play(Sfx::Ui);
        }

        let character = &CHARACTERS[self.roster_index];
        render::draw_text_px(
            assets,
            &format!(
                "{}  ·  {}  ·  {:.0} MANA",
                character.name.to_uppercase(),
                character.ability.name.to_uppercase(),
                character.ability.cost
            ),
            W / 2.0,
            330.0,
            0.36,
            render::rgba([255, 238, 190], 1.0),
            TextAlign::Center,
            true,
        );
        let wrapped = render::wrap_text(assets, character.ability.desc, 92, 0.26);
        render::draw_text_px(
            assets,
            &wrapped,
            W / 2.0,
            350.0,
            0.26,
            render::rgba([186, 199, 218], 0.95),
            TextAlign::Center,
            false,
        );
        render::draw_text_px(
            assets,
            &format!(
                "{}  ·  {}",
                character.ability.category.label().to_uppercase(),
                character.blurb.to_uppercase()
            ),
            W / 2.0,
            378.0,
            0.2,
            render::rgba([132, 146, 166], 0.9),
            TextAlign::Center,
            false,
        );
    }

    fn choose_roster(&mut self, audio: &Audio) {
        if self.roster_slot == 0 {
            self.setup.p1 = self.roster_index;
        } else {
            self.setup.p2 = self.roster_index;
        }
        self.screen = Screen::Setup;
        self.setup_focus = self.roster_slot;
        audio.play(Sfx::Ui);
    }

    fn screen_arena(&mut self, assets: &Assets, audio: &Audio, pointer: &Pointer, nav: &Nav) {
        draw_header(assets, "CHOOSE ARENA", "ELEVEN PLACES TO SETTLE IT");
        let cols = 4;
        let cell_w = 128.0;
        let cell_h = 90.0;
        let start_x = 24.0;
        let start_y = 46.0;
        if nav.left {
            self.arena_index = (self.arena_index + ARENAS.len() - 1) % ARENAS.len();
            audio.play(Sfx::Ui);
        }
        if nav.right {
            self.arena_index = (self.arena_index + 1) % ARENAS.len();
            audio.play(Sfx::Ui);
        }
        if nav.up {
            self.arena_index = (self.arena_index + ARENAS.len() - cols) % ARENAS.len();
            audio.play(Sfx::Ui);
        }
        if nav.down {
            self.arena_index = (self.arena_index + cols) % ARENAS.len();
            audio.play(Sfx::Ui);
        }

        for (index, arena) in ARENAS.iter().enumerate() {
            let col = index % cols;
            let row = index / cols;
            let x = start_x + col as f32 * cell_w;
            let y = start_y + row as f32 * cell_h;
            let hovered = pointer.over(x + 1.0, y + 1.0, cell_w - 4.0, cell_h - 14.0);
            if hovered {
                self.arena_index = index;
            }
            let selected = self.arena_index == index;
            render::panel(
                x + 1.0,
                y + 1.0,
                cell_w - 4.0,
                cell_h - 4.0,
                render::rgba([10, 16, 26], 0.9),
                if selected {
                    render::rgba([255, 212, 90], 1.0)
                } else {
                    render::rgba([44, 55, 72], 0.8)
                },
            );
            draw_texture_ex(
                &assets.backgrounds[index],
                x + 4.0,
                y + 4.0,
                if selected {
                    WHITE
                } else {
                    render::rgba([255, 255, 255], 0.82)
                },
                DrawTextureParams {
                    dest_size: Some(vec2(cell_w - 10.0, cell_h - 22.0)),
                    ..Default::default()
                },
            );
            render::draw_text_px(
                assets,
                &short_name(arena.name, 16),
                x + (cell_w - 4.0) / 2.0,
                y + cell_h - 17.0,
                0.22,
                if selected {
                    render::rgba([255, 244, 214], 1.0)
                } else {
                    render::rgba([172, 186, 206], 0.95)
                },
                TextAlign::Center,
                false,
            );
            if pointer.hit(x + 1.0, y + 1.0, cell_w - 4.0, cell_h - 4.0) {
                self.arena_index = index;
                self.setup.arena = index;
                self.screen = Screen::Setup;
                self.setup_focus = 3;
                audio.play(Sfx::Ui);
                return;
            }
        }

        if nav.confirm {
            self.setup.arena = self.arena_index;
            self.screen = Screen::Setup;
            self.setup_focus = 3;
            audio.play(Sfx::Ui);
            return;
        }
        let back = draw_back_button(assets, pointer);
        if nav.back || pointer.hit(back.0, back.1, back.2, back.3) {
            self.screen = Screen::Setup;
            self.setup_focus = 3;
            audio.play(Sfx::Ui);
        }
        let arena = &ARENAS[self.arena_index];
        render::draw_text_px(
            assets,
            &arena.name.to_uppercase(),
            W / 2.0,
            334.0,
            0.4,
            render::rgba([255, 240, 196], 1.0),
            TextAlign::Center,
            true,
        );
        render::draw_text_px(
            assets,
            &arena.desc.to_uppercase(),
            W / 2.0,
            356.0,
            0.24,
            render::rgba([182, 195, 214], 0.95),
            TextAlign::Center,
            false,
        );
    }

    fn screen_settings(
        &mut self,
        assets: &Assets,
        save: &mut SaveData,
        audio: &Audio,
        pointer: &Pointer,
        nav: &Nav,
    ) {
        draw_header(assets, "SETTINGS", "AUDIO, SCREEN AND CONTROLS");
        let accent = render::rgba([255, 212, 90], 1.0);
        let panel_x = 56.0;
        let panel_w = W - 112.0;
        render::panel(
            panel_x,
            46.0,
            panel_w,
            296.0,
            render::rgba([10, 15, 24], 0.9),
            render::rgba([44, 56, 74], 0.9),
        );
        let row_x = panel_x + 16.0;
        let row_w = panel_w - 32.0;
        let start_y = 58.0;
        let row_h = 34.0;
        let labels = [
            "MUSIC",
            "SOUND FX",
            "SCREEN SHAKE",
            "SHOW FPS",
            "LANGUAGE",
            "CONTROL BINDINGS",
            "BACK",
        ];
        for (i, label) in labels.iter().enumerate() {
            let y = start_y + i as f32 * row_h;
            let focused = self.settings_focus == i;
            let hovered = pointer.over(row_x, y, row_w, row_h - 4.0);
            if hovered && pointer.click {
                self.settings_focus = i;
            }
            let fill = if focused || hovered {
                render::rgba([24, 34, 48], 0.95)
            } else {
                render::rgba([15, 22, 33], 0.85)
            };
            render::panel(
                row_x,
                y,
                row_w,
                row_h - 4.0,
                fill,
                if focused || hovered {
                    accent
                } else {
                    render::rgba([42, 54, 72], 0.8)
                },
            );
            render::draw_text_px(
                assets,
                label,
                row_x + 10.0,
                y + 9.0,
                0.3,
                if focused {
                    render::rgba([255, 244, 214], 1.0)
                } else {
                    render::rgba([208, 220, 236], 0.95)
                },
                TextAlign::Left,
                false,
            );
            match i {
                0 | 1 => {
                    let value = if i == 0 {
                        save.settings.music
                    } else {
                        save.settings.sfx
                    };
                    let bar_x = row_x + 190.0;
                    let bar_w = row_w - 250.0;
                    let bar_y = y + 12.0;
                    draw_rectangle(bar_x, bar_y, bar_w, 8.0, render::rgba([8, 12, 19], 1.0));
                    draw_rectangle(
                        bar_x,
                        bar_y,
                        bar_w * value,
                        8.0,
                        render::rgba([142, 230, 79], 1.0),
                    );
                    draw_rectangle_lines(
                        bar_x,
                        bar_y,
                        bar_w,
                        8.0,
                        1.0,
                        render::rgba([72, 88, 110], 1.0),
                    );
                    let knob = bar_x + bar_w * value;
                    draw_rectangle(knob - 2.0, bar_y - 2.0, 4.0, 12.0, WHITE);
                    render::draw_text_px(
                        assets,
                        &format!("{:.0}%", value * 100.0),
                        row_x + row_w - 12.0,
                        y + 8.0,
                        0.28,
                        render::rgba([255, 236, 184], 1.0),
                        TextAlign::Right,
                        false,
                    );
                    let bar_rect = (bar_x, bar_y - 4.0, bar_w, 16.0);
                    if (pointer.click || pointer.down)
                        && pointer.over(bar_rect.0, bar_rect.1, bar_rect.2, bar_rect.3)
                    {
                        let value = ((pointer.x - bar_x) / bar_w).clamp(0.0, 1.0);
                        if i == 0 {
                            save.settings.music = value;
                        } else {
                            save.settings.sfx = value;
                        }
                        self.dirty = true;
                    }
                }
                2 => {
                    let on = save.settings.shake;
                    draw_toggle(assets, row_x + row_w - 70.0, y + 7.0, on);
                    if (pointer.click && hovered) || (self.settings_focus == 2 && nav.confirm) {
                        save.settings.shake = !on;
                        self.dirty = true;
                        audio.play(Sfx::Ui);
                    }
                }
                3 => {
                    let on = save.settings.show_fps;
                    draw_toggle(assets, row_x + row_w - 70.0, y + 7.0, on);
                    if (pointer.click && hovered) || (self.settings_focus == 3 && nav.confirm) {
                        save.settings.show_fps = !on;
                        self.dirty = true;
                        audio.play(Sfx::Ui);
                    }
                }
                4 => {
                    let value = i18n::localize(save.settings.language.label());
                    render::draw_text_px(
                        assets,
                        &value,
                        row_x + row_w - 12.0,
                        y + 8.0,
                        0.28,
                        render::rgba([255, 236, 184], 1.0),
                        TextAlign::Right,
                        false,
                    );
                    if (pointer.click && hovered) || (self.settings_focus == 4 && nav.confirm) {
                        save.settings.language = save.settings.language.toggled();
                        i18n::set_language(save.settings.language);
                        self.dirty = true;
                        audio.play(Sfx::Ui);
                    }
                }
                5 => {
                    if (pointer.click && hovered) || (self.settings_focus == 5 && nav.confirm) {
                        self.settings_focus = 5;
                        self.screen = Screen::Controls;
                        audio.play(Sfx::Ui);
                    }
                    render::draw_text_px(
                        assets,
                        ">",
                        row_x + row_w - 18.0,
                        y + 5.0,
                        0.4,
                        render::rgba([255, 212, 90], 0.9),
                        TextAlign::Center,
                        false,
                    );
                }
                6 if (pointer.click && hovered) || (self.settings_focus == 6 && nav.confirm) => {
                    self.screen = Screen::Title;
                    audio.play(Sfx::Ui);
                }
                _ => {}
            }
        }

        if nav.up {
            self.settings_focus = (self.settings_focus + 6) % 7;
            audio.play(Sfx::Ui);
        }
        if nav.down {
            self.settings_focus = (self.settings_focus + 1) % 7;
            audio.play(Sfx::Ui);
        }
        if nav.left || nav.right {
            let dir = if nav.right { 0.05 } else { -0.05 };
            match self.settings_focus {
                0 => {
                    save.settings.music = (save.settings.music + dir).clamp(0.0, 1.0);
                    self.dirty = true;
                }
                1 => {
                    save.settings.sfx = (save.settings.sfx + dir).clamp(0.0, 1.0);
                    self.dirty = true;
                }
                4 => {
                    save.settings.language = save.settings.language.toggled();
                    i18n::set_language(save.settings.language);
                    self.dirty = true;
                    audio.play(Sfx::Ui);
                }
                _ => {}
            }
        }
        if nav.back {
            self.screen = Screen::Title;
            audio.play(Sfx::Ui);
        }

        let progress = &save.progress;
        render::draw_text_px(
            assets,
            &format!(
                "MATCHES {}   WINS {}   CUPS {}   GOALS {}",
                progress.matches, progress.wins, progress.championships, progress.goals
            ),
            W / 2.0,
            352.0,
            0.26,
            render::rgba([150, 165, 184], 0.95),
            TextAlign::Center,
            false,
        );
        render::draw_text_px(
            assets,
            "UP/DOWN SELECT  ·  LEFT/RIGHT ADJUST  ·  ENTER CONFIRM",
            W / 2.0,
            374.0,
            0.22,
            render::rgba([120, 134, 154], 0.9),
            TextAlign::Center,
            false,
        );
    }

    fn screen_controls(
        &mut self,
        assets: &Assets,
        save: &mut SaveData,
        audio: &Audio,
        pointer: &Pointer,
        nav: &Nav,
    ) {
        draw_header(assets, "CONTROL BINDINGS", "CLICK A ROW, THEN PRESS A KEY");
        let panel_x = 40.0;
        let panel_w = W - 80.0;
        render::panel(
            panel_x,
            44.0,
            panel_w,
            310.0,
            render::rgba([10, 15, 24], 0.92),
            render::rgba([44, 56, 74], 0.9),
        );
        let labels = ["LEFT", "RIGHT", "JUMP", "ABILITY"];
        for row in 0..8 {
            let player = row / 4;
            let field = row % 4;
            let controls = if player == 0 {
                &save.settings.p1
            } else {
                &save.settings.p2
            };
            let key = match field {
                0 => controls.left,
                1 => controls.right,
                2 => controls.jump,
                _ => controls.ability,
            };
            let x = panel_x + 16.0;
            let y = 54.0 + row as f32 * 28.0;
            let w = panel_w - 32.0;
            let focused = self.controls_focus == row;
            let hovered = pointer.over(x, y, w, 24.0);
            let capture = self.capture == Some(row);
            render::panel(
                x,
                y,
                w,
                24.0,
                if focused || hovered || capture {
                    render::rgba([24, 34, 48], 0.95)
                } else {
                    render::rgba([15, 22, 33], 0.85)
                },
                if capture {
                    render::rgba([255, 120, 120], 1.0)
                } else if focused || hovered {
                    render::side_color(player)
                } else {
                    render::rgba([42, 54, 72], 0.8)
                },
            );
            render::draw_text_px(
                assets,
                &format!("P{}  {}", player + 1, labels[field]),
                x + 10.0,
                y + 4.0,
                0.28,
                render::side_color(player),
                TextAlign::Left,
                false,
            );
            let key_label = if capture {
                "PRESS A KEY...".to_string()
            } else {
                key.name()
            };
            render::draw_text_px(
                assets,
                &key_label,
                x + w - 12.0,
                y + 4.0,
                0.3,
                if capture {
                    render::rgba([255, 210, 150], 1.0)
                } else {
                    render::rgba([235, 242, 252], 1.0)
                },
                TextAlign::Right,
                false,
            );
            if pointer.hit(x, y, w, 24.0) {
                self.controls_focus = row;
                self.capture = Some(row);
                audio.play(Sfx::Ui);
            }
        }

        let reset = (W / 2.0 - 150.0, 292.0, 140.0, 26.0);
        let back = (W / 2.0 + 10.0, 292.0, 140.0, 26.0);
        draw_button(
            assets,
            "RESET DEFAULTS",
            reset.0,
            reset.1,
            reset.2,
            reset.3,
            self.controls_focus == 8,
            pointer.over(reset.0, reset.1, reset.2, reset.3),
            render::rgba([255, 212, 90], 1.0),
        );
        draw_button(
            assets,
            "DONE",
            back.0,
            back.1,
            back.2,
            back.3,
            self.controls_focus == 9,
            pointer.over(back.0, back.1, back.2, back.3),
            render::rgba([142, 230, 79], 1.0),
        );

        if let Some(row) = self.capture {
            if is_key_pressed(KeyCode::Escape) {
                self.capture = None;
                audio.play(Sfx::Ui);
            } else if let Some(key) = get_keys_pressed().iter().next().copied() {
                if key != KeyCode::Escape {
                    let player = row / 4;
                    let field = row % 4;
                    let controls = if player == 0 {
                        &mut save.settings.p1
                    } else {
                        &mut save.settings.p2
                    };
                    let code = GameKey::from_code(key as u16);
                    match field {
                        0 => controls.left = code,
                        1 => controls.right = code,
                        2 => controls.jump = code,
                        _ => controls.ability = code,
                    }
                    self.capture = None;
                    self.dirty = true;
                    audio.play(Sfx::Ui);
                }
            }
            if nav.back {
                self.capture = None;
            }
            return;
        }

        if nav.up {
            self.controls_focus = (self.controls_focus + 9) % 10;
            audio.play(Sfx::Ui);
        }
        if nav.down {
            self.controls_focus = (self.controls_focus + 1) % 10;
            audio.play(Sfx::Ui);
        }
        if nav.confirm {
            match self.controls_focus {
                0..=7 => {
                    self.capture = Some(self.controls_focus);
                    audio.play(Sfx::Ui);
                }
                8 => {
                    save.settings.p1 = Controls::default();
                    save.settings.p2 = Controls::wasd();
                    self.dirty = true;
                    audio.play(Sfx::Ui);
                }
                _ => {
                    self.screen = Screen::Settings;
                    self.settings_focus = 5;
                    audio.play(Sfx::Ui);
                }
            }
        }
        if pointer.hit(reset.0, reset.1, reset.2, reset.3) {
            save.settings.p1 = Controls::default();
            save.settings.p2 = Controls::wasd();
            self.dirty = true;
            audio.play(Sfx::Ui);
        }
        if pointer.hit(back.0, back.1, back.2, back.3) {
            self.screen = Screen::Settings;
            self.settings_focus = 5;
            audio.play(Sfx::Ui);
        }
        if nav.back {
            self.screen = Screen::Settings;
            self.settings_focus = 5;
            audio.play(Sfx::Ui);
        }
    }

    fn screen_help(&mut self, assets: &Assets, audio: &Audio, pointer: &Pointer, nav: &Nav) {
        draw_header(assets, "HOW TO PLAY", "THREE SPORTS. ONE BRAINCELL.");
        let left_x = 26.0;
        let right_x = 292.0;
        let mut y = 52.0;
        let green = render::rgba([142, 230, 79], 1.0);
        let yellow = render::rgba([255, 212, 90], 1.0);
        render::draw_text_px(
            assets,
            "CONTROLS",
            left_x,
            y,
            0.32,
            green,
            TextAlign::Left,
            false,
        );
        y += 20.0;
        for line in [
            "P1  ARROWS + SPACE",
            "P2  A / D + W + LEFT CTRL",
            "PAUSE  ESC",
            "ABILITY  ONE PER SLIME",
        ] {
            render::draw_text_px(
                assets,
                line,
                left_x,
                y,
                0.24,
                render::rgba([212, 224, 240], 0.95),
                TextAlign::Left,
                false,
            );
            y += 15.0;
        }
        y += 12.0;
        render::draw_text_px(
            assets,
            "MATCH FLOW",
            left_x,
            y,
            0.32,
            green,
            TextAlign::Left,
            false,
        );
        y += 20.0;
        for line in [
            "FIRST TO THE TARGET SCORE WINS.",
            "AFTER EACH POINT BOTH SIDES RESET.",
            "MANA REFILLS BY MODE. BONUS",
            "REWARDS DEFENDING YOUR HALF.",
        ] {
            render::draw_text_px(
                assets,
                line,
                left_x,
                y,
                0.24,
                render::rgba([196, 209, 226], 0.95),
                TextAlign::Left,
                false,
            );
            y += 15.0;
        }

        let mut y = 52.0;
        render::draw_text_px(
            assets,
            "SOCCER",
            right_x,
            y,
            0.32,
            yellow,
            TextAlign::Left,
            false,
        );
        y += 20.0;
        let soccer = render::wrap_text(
            assets,
            "ROLL THE BALL ACROSS THE GOAL LINE. THE CROSSBAR AND POSTS ARE SOLID.",
            34,
            0.23,
        );
        render::draw_text_px(
            assets,
            &soccer,
            right_x,
            y,
            0.23,
            render::rgba([196, 209, 226], 0.95),
            TextAlign::Left,
            false,
        );
        y += 50.0;
        render::draw_text_px(
            assets,
            "VOLLEYBALL",
            right_x,
            y,
            0.32,
            yellow,
            TextAlign::Left,
            false,
        );
        y += 20.0;
        let volley = render::wrap_text(
            assets,
            "KEEP THE BALL OFF YOUR FLOOR. THE CENTER NET BLOCKS EVERYTHING, INCLUDING DIGNITY.",
            34,
            0.23,
        );
        render::draw_text_px(
            assets,
            &volley,
            right_x,
            y,
            0.23,
            render::rgba([196, 209, 226], 0.95),
            TextAlign::Left,
            false,
        );
        y += 50.0;
        render::draw_text_px(
            assets,
            "BASKETBALL",
            right_x,
            y,
            0.32,
            yellow,
            TextAlign::Left,
            false,
        );
        y += 20.0;
        let basket = render::wrap_text(
            assets,
            "DROP THE BALL THROUGH THE RIM FROM ABOVE. RIM SHOTS DO NOT COUNT.",
            34,
            0.23,
        );
        render::draw_text_px(
            assets,
            &basket,
            right_x,
            y,
            0.23,
            render::rgba([196, 209, 226], 0.95),
            TextAlign::Left,
            false,
        );
        y += 50.0;
        render::draw_text_px(
            assets,
            "EVERY SLIME HAS A UNIQUE ABILITY.",
            right_x,
            y,
            0.24,
            green,
            TextAlign::Left,
            false,
        );
        y += 16.0;
        render::draw_text_px(
            assets,
            "CHAMPIONSHIP: WIN THREE IN A ROW.",
            right_x,
            y,
            0.24,
            render::rgba([255, 226, 150], 0.95),
            TextAlign::Left,
            false,
        );

        let back = draw_back_button(assets, pointer);
        if nav.back || pointer.hit(back.0, back.1, back.2, back.3) {
            self.screen = Screen::Title;
            audio.play(Sfx::Ui);
        }
        render::draw_text_px(
            assets,
            "SPACE / ENTER CONFIRM  ·  ESC BACK",
            W / 2.0,
            374.0,
            0.22,
            render::rgba([128, 142, 162], 0.9),
            TextAlign::Center,
            false,
        );
    }

    fn screen_results(
        &mut self,
        assets: &Assets,
        save: &mut SaveData,
        audio: &Audio,
        pointer: &Pointer,
        nav: &Nav,
    ) {
        let Some(report) = self.last_report.clone() else {
            self.screen = Screen::Title;
            return;
        };
        let mode = report.request.mode;
        let run_done = self.championship.as_ref().map(|r| r.done).unwrap_or(false);
        let run_won = self.championship.as_ref().map(|r| r.won).unwrap_or(false);

        let headline = if mode == LaunchMode::Championship {
            if run_done && run_won {
                "CHAMPION!"
            } else if run_done {
                "ELIMINATED"
            } else {
                "ROUND CLEARED"
            }
        } else if mode == LaunchMode::Online {
            if report.winner == report.request.local_side {
                "YOU WIN"
            } else {
                "YOU LOSE"
            }
        } else if report.winner == 0 {
            if report.request.ai {
                "YOU WIN"
            } else {
                "PLAYER 1 WINS"
            }
        } else if report.request.ai {
            "YOU LOSE"
        } else {
            "PLAYER 2 WINS"
        };
        let headline_color = if mode == LaunchMode::Championship && run_done && !run_won {
            render::rgba([255, 118, 128], 1.0)
        } else {
            render::side_color(report.winner)
        };
        render::draw_text_px(
            assets,
            headline,
            W / 2.0,
            22.0,
            1.0,
            headline_color,
            TextAlign::Center,
            true,
        );

        let p1 = &CHARACTERS[report.request.config.p1];
        let p2 = &CHARACTERS[report.request.config.p2];
        render::draw_text_px(
            assets,
            &format!(
                "{}   {}  -  {}   {}",
                p1.name.to_uppercase(),
                report.score[0],
                report.score[1],
                p2.name.to_uppercase()
            ),
            W / 2.0,
            78.0,
            0.46,
            render::rgba([248, 251, 255], 1.0),
            TextAlign::Center,
            true,
        );
        render::draw_text_px(
            assets,
            &format!(
                "{}  ·  {}  ·  {:.0}s",
                report.request.config.sport.label().to_uppercase(),
                ARENAS[report.request.config.arena].name.to_uppercase(),
                report.elapsed
            ),
            W / 2.0,
            104.0,
            0.26,
            render::rgba([168, 182, 202], 0.95),
            TextAlign::Center,
            false,
        );

        render::panel(
            52.0,
            128.0,
            W - 104.0,
            104.0,
            render::rgba([10, 15, 24], 0.86),
            render::rgba([44, 56, 74], 0.9),
        );
        let rows = [
            (
                "HITS",
                report.stats.hits[0].to_string(),
                report.stats.hits[1].to_string(),
            ),
            (
                "ABILITIES",
                report.stats.abilities[0].to_string(),
                report.stats.abilities[1].to_string(),
            ),
            (
                "TOP BALL SPEED",
                format!("{:.0}", report.stats.top_speed),
                String::new(),
            ),
        ];
        render::draw_text_px(
            assets,
            "STATS",
            68.0,
            138.0,
            0.26,
            render::rgba([150, 166, 186], 0.95),
            TextAlign::Left,
            false,
        );
        render::draw_text_px(
            assets,
            "P1",
            W / 2.0 - 38.0,
            138.0,
            0.26,
            render::side_color(0),
            TextAlign::Center,
            false,
        );
        render::draw_text_px(
            assets,
            "P2",
            W / 2.0 + 38.0,
            138.0,
            0.26,
            render::side_color(1),
            TextAlign::Center,
            false,
        );
        for (i, (label, a, b)) in rows.iter().enumerate() {
            let y = 160.0 + i as f32 * 21.0;
            render::draw_text_px(
                assets,
                label,
                68.0,
                y,
                0.26,
                render::rgba([198, 210, 226], 0.95),
                TextAlign::Left,
                false,
            );
            render::draw_text_px(
                assets,
                a,
                W / 2.0 - 38.0,
                y,
                0.3,
                render::rgba([240, 246, 255], 1.0),
                TextAlign::Center,
                false,
            );
            if !b.is_empty() {
                render::draw_text_px(
                    assets,
                    b,
                    W / 2.0 + 38.0,
                    y,
                    0.3,
                    render::rgba([240, 246, 255], 1.0),
                    TextAlign::Center,
                    false,
                );
            }
        }

        let buttons: Vec<(&str, ResultAction)> = if mode == LaunchMode::Online {
            vec![
                ("ONLINE MENU", ResultAction::OnlineMenu),
                ("MENU", ResultAction::Menu),
            ]
        } else if mode == LaunchMode::Casual {
            vec![
                ("REMATCH", ResultAction::Rematch),
                ("CHANGE SETUP", ResultAction::ChangeSetup),
                ("MENU", ResultAction::Menu),
            ]
        } else if run_done && run_won {
            vec![
                ("NEW RUN", ResultAction::NewRun),
                ("CHANGE SETUP", ResultAction::ChangeSetup),
                ("MENU", ResultAction::Menu),
            ]
        } else if run_done {
            vec![
                ("TRY AGAIN", ResultAction::NewRun),
                ("CHANGE SETUP", ResultAction::ChangeSetup),
                ("MENU", ResultAction::Menu),
            ]
        } else {
            vec![
                ("NEXT ROUND", ResultAction::NextRound),
                ("CHANGE SETUP", ResultAction::ChangeSetup),
                ("MENU", ResultAction::Menu),
            ]
        };
        let count = buttons.len();
        let button_w = 130.0;
        let gap = 10.0;
        let total = count as f32 * button_w + (count as f32 - 1.0) * gap;
        let start_x = W / 2.0 - total / 2.0;
        let button_y = 284.0;
        for (i, (label, _)) in buttons.iter().enumerate() {
            let x = start_x + i as f32 * (button_w + gap);
            let hover = pointer.over(x, button_y, button_w, 32.0);
            draw_button(
                assets,
                label,
                x,
                button_y,
                button_w,
                32.0,
                self.result_focus == i,
                hover,
                render::rgba([255, 212, 90], 1.0),
            );
        }

        if mode == LaunchMode::Championship {
            if let Some(run) = &self.championship {
                let y = 336.0;
                render::draw_text_px(
                    assets,
                    &format!(
                        "CUP RUN  ·  ROUND {} OF {}",
                        (run.round + 1).min(3),
                        run.opponents.len()
                    ),
                    W / 2.0,
                    y,
                    0.26,
                    render::rgba([255, 226, 150], 0.95),
                    TextAlign::Center,
                    false,
                );
                for i in 0..3 {
                    let x = W / 2.0 - 24.0 + i as f32 * 18.0;
                    let cleared = i < run.scores.len() && run.scores[i][0] > run.scores[i][1];
                    draw_circle(
                        x,
                        y + 18.0,
                        4.0,
                        if cleared {
                            render::rgba([142, 230, 79], 1.0)
                        } else {
                            render::rgba([80, 94, 112], 1.0)
                        },
                    );
                }
            }
        }

        if nav.right {
            self.result_focus = (self.result_focus + 1) % count;
            audio.play(Sfx::Ui);
        }
        if nav.left {
            self.result_focus = (self.result_focus + count - 1) % count;
            audio.play(Sfx::Ui);
        }
        let clicked = (0..count).find(|i| {
            let x = start_x + *i as f32 * (button_w + gap);
            pointer.hit(x, button_y, button_w, 32.0)
        });
        if nav.confirm || clicked.is_some() {
            if let Some(index) = clicked {
                self.result_focus = index;
            }
            let action = buttons[self.result_focus.min(count - 1)].1;
            self.activate_result(action, &report, save, audio);
        }
        if nav.back {
            self.screen = Screen::Title;
            self.championship = None;
            audio.play(Sfx::Ui);
        }
        render::draw_text_px(
            assets,
            "ENTER SELECT  ·  ESC MENU",
            W / 2.0,
            374.0,
            0.22,
            render::rgba([128, 142, 162], 0.9),
            TextAlign::Center,
            false,
        );
    }

    fn activate_result(
        &mut self,
        action: ResultAction,
        report: &MatchReport,
        save: &mut SaveData,
        audio: &Audio,
    ) {
        audio.play(Sfx::Ui);
        match action {
            ResultAction::Rematch => {
                let mut request = report.request.clone();
                request.config.seed = seed_now();
                self.pending_action = Some(UiAction::Launch(request));
            }
            ResultAction::NextRound => {
                if self.championship.as_ref().map(|r| r.done).unwrap_or(true) {
                    self.new_championship_run(save, audio);
                } else {
                    self.launch_championship_round();
                }
            }
            ResultAction::NewRun => self.new_championship_run(save, audio),
            ResultAction::OnlineMenu => {
                self.to_online();
                self.notice = Some("MATCH COMPLETE".into());
            }
            ResultAction::ChangeSetup => {
                self.screen = Screen::Setup;
                self.championship_setup = report.request.mode == LaunchMode::Championship;
                self.setup_focus = 0;
            }
            ResultAction::Menu => {
                self.screen = Screen::Title;
                self.championship = None;
            }
        }
    }

    fn new_championship_run(&mut self, save: &mut SaveData, audio: &Audio) {
        if let Some(run) = &self.championship {
            self.setup.sport = run.sport;
            self.setup.arena = run.arena;
            self.setup.p1 = run.player;
            self.setup.difficulty = run.difficulty;
        }
        self.championship_setup = true;
        self.setup.local2p = false;
        self.championship = None;
        self.start_championship(save, audio);
    }
}

#[derive(Clone, Copy, Debug)]
enum ResultAction {
    Rematch,
    NextRound,
    NewRun,
    OnlineMenu,
    ChangeSetup,
    Menu,
}

fn nav_confirm_opens_roster(focus: usize, championship: bool, online: bool) -> bool {
    focus == 0 || (focus == 1 && !championship && !online)
}

fn draw_toggle(assets: &Assets, x: f32, y: f32, on: bool) {
    draw_rectangle(
        x,
        y,
        52.0,
        18.0,
        if on {
            render::rgba([38, 82, 48], 1.0)
        } else {
            render::rgba([30, 37, 48], 1.0)
        },
    );
    draw_rectangle_lines(
        x,
        y,
        52.0,
        18.0,
        1.0,
        if on {
            render::rgba([142, 230, 79], 1.0)
        } else {
            render::rgba([82, 94, 112], 1.0)
        },
    );
    draw_rectangle(
        x + if on { 36.0 } else { 2.0 },
        y + 2.0,
        14.0,
        14.0,
        if on {
            render::rgba([142, 230, 79], 1.0)
        } else {
            render::rgba([130, 142, 158], 1.0)
        },
    );
    render::draw_text_px(
        assets,
        if on { "ON" } else { "OFF" },
        x + 26.0,
        y + 3.0,
        0.2,
        render::rgba([236, 242, 250], 1.0),
        TextAlign::Center,
        false,
    );
}
