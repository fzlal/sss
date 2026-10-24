use macroquad::prelude::*;

use crate::assets::Assets;
use crate::audio::{Audio, Sfx};
use crate::game::ai::AIController;
use crate::game::arenas::ARENAS;
use crate::game::characters::CHARACTERS;
use crate::game::config::{Difficulty, InputFrame, ManaMode, Sport, H, W};
use crate::game::sim::{Event, Match, MatchConfig, Phase};
use crate::i18n;
use crate::net::cli::Cli;
use crate::net::{
    self, event_code as codes, Client, ClientStatus, Server, ServerMode, ServerStatus, DEFAULT_PORT,
};
use crate::render::{self, Canvas, TextAlign};
use crate::save::{Controls, SaveData};
use crate::ui::{FrontEnd, LaunchMode, LaunchRequest, MatchReport, UiAction};

const STEP: f32 = 1.0 / 120.0;
const MAX_STEPS: u32 = 16;
const END_HOLD: f32 = 1.15;

struct MatchSession {
    m: Match,
    request: LaunchRequest,
    ai: Option<AIController>,
    paused: bool,
    end_timer: f32,
    accumulator: f32,
}

impl MatchSession {
    fn new(request: LaunchRequest) -> Self {
        let ai = if request.ai {
            Some(AIController::new(
                request.difficulty,
                1,
                request.config.seed ^ 0x51ed_2701_9e37_79b9,
            ))
        } else {
            None
        };
        let m = Match::new(request.config);
        Self {
            m,
            request,
            ai,
            paused: false,
            end_timer: 0.0,
            accumulator: 0.0,
        }
    }

    fn human_input(controls: &Controls) -> InputFrame {
        InputFrame {
            left: controls.left.down(),
            right: controls.right.down(),
            jump: controls.jump.down(),
            ability: controls.ability.down(),
        }
    }

    fn report(&self) -> MatchReport {
        let winner = self.m.winner.unwrap_or_else(|| {
            if self.m.score[0] >= self.m.score[1] {
                0
            } else {
                1
            }
        });
        MatchReport {
            request: self.request.clone(),
            score: self.m.score,
            winner,
            stats: self.m.stats.clone(),
            elapsed: self.m.elapsed,
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum OnlineSession {
    Host {
        server: Server,
        request: LaunchRequest,
        end_timer: f32,
    },
    Client {
        client: Client,
        request: LaunchRequest,
        char_id: usize,
        end_timer: f32,
        end_sound: bool,
    },
}

enum OnlineOutcome {
    Finished(MatchReport),
    Left(String),
}

fn online_request(config: MatchConfig, difficulty: Difficulty, local_side: usize) -> LaunchRequest {
    LaunchRequest {
        config,
        ai: false,
        difficulty,
        mode: LaunchMode::Online,
        local_side,
    }
}

fn placeholder_config(char_id: usize) -> MatchConfig {
    MatchConfig {
        sport: Sport::Soccer,
        arena: 0,
        p1: char_id.min(CHARACTERS.len() - 1),
        p2: 4.min(CHARACTERS.len() - 1),
        play_to: 6,
        mana_mode: ManaMode::Bonus,
        seed: 0,
        p1_control: 1.0,
        p2_control: 1.0,
    }
}

fn start_host(
    config: MatchConfig,
    difficulty: Difficulty,
    port: u16,
    name: &str,
) -> Result<OnlineSession, String> {
    let server = Server::new(config, ServerMode::Hosted, port, name.to_string())
        .map_err(|error| format!("CANNOT HOST ON PORT {port}: {error}"))?;
    Ok(OnlineSession::Host {
        server,
        request: online_request(config, difficulty, 0),
        end_timer: 0.0,
    })
}

fn start_client(addr: &str, char_id: usize, name: &str) -> Result<OnlineSession, String> {
    let client =
        Client::connect(addr, name, char_id).map_err(|error| format!("CANNOT CONNECT: {error}"))?;
    Ok(OnlineSession::Client {
        client,
        request: online_request(placeholder_config(char_id), Difficulty::Normal, 0),
        char_id,
        end_timer: 0.0,
        end_sound: false,
    })
}

fn play_event(audio: &Audio, event: &Event, ai: bool, local_side: Option<usize>) {
    match event {
        Event::Countdown(_) => audio.play(Sfx::Countdown),
        Event::Go => audio.play(Sfx::Go),
        Event::Jump { .. } => audio.play(Sfx::Jump),
        Event::Kick { head, .. } => {
            if *head {
                audio.play(Sfx::Head);
            } else {
                audio.play(Sfx::Kick);
            }
        }
        Event::Bounce { kind, .. } => audio.play(match *kind {
            "net" => Sfx::Net,
            "post" | "rim" => Sfx::Post,
            _ => Sfx::Bounce,
        }),
        Event::Ability { .. } => audio.play(Sfx::Ability),
        Event::Deny { .. } => audio.play(Sfx::Deny),
        Event::Score { sport, .. } => {
            audio.play(match sport {
                Sport::Soccer => Sfx::Goal,
                Sport::Volleyball => Sfx::Net,
                Sport::Basketball => Sfx::Basket,
            });
        }
        Event::Whistle { .. } => audio.play(Sfx::Whistle),
        Event::MatchEnd { winner, .. } => {
            let won = match local_side {
                Some(side) => *winner == side,
                None => !ai || *winner == 0,
            };
            audio.play(if won { Sfx::Win } else { Sfx::Lose });
        }
        Event::Explosion => audio.play(Sfx::Explosion),
        Event::Geyser { .. } => audio.play(Sfx::Bounce),
        Event::DiscoHit { .. } | Event::ProjectileHit { .. } | Event::WaveHit { .. } => {
            audio.play(Sfx::Kick)
        }
    }
}

fn play_code(audio: &Audio, code: u8) {
    match code {
        codes::COUNTDOWN => audio.play(Sfx::Countdown),
        codes::GO => audio.play(Sfx::Go),
        codes::JUMP => audio.play(Sfx::Jump),
        codes::KICK => audio.play(Sfx::Kick),
        codes::HEAD => audio.play(Sfx::Head),
        codes::BOUNCE => audio.play(Sfx::Bounce),
        codes::NET => audio.play(Sfx::Net),
        codes::POST => audio.play(Sfx::Post),
        codes::ABILITY => audio.play(Sfx::Ability),
        codes::DENY => audio.play(Sfx::Deny),
        codes::WHISTLE => audio.play(Sfx::Whistle),
        codes::EXPLOSION => audio.play(Sfx::Explosion),
        codes::SCORE_SOCCER => audio.play(Sfx::Goal),
        codes::SCORE_VOLLEY => audio.play(Sfx::Net),
        codes::SCORE_BASKET => audio.play(Sfx::Basket),
        _ => {}
    }
}

fn update_online(
    session: &mut OnlineSession,
    dt: f32,
    input: InputFrame,
    paused: bool,
    audio: &Audio,
) -> Option<OnlineOutcome> {
    let input = if paused { InputFrame::NONE } else { input };
    match session {
        OnlineSession::Host {
            server,
            request,
            end_timer,
        } => {
            server.update(dt, Some(input));
            for event in server.take_events() {
                play_event(audio, &event, false, Some(0));
            }
            if let ServerStatus::Aborted(reason) = server.status() {
                return Some(OnlineOutcome::Left(reason.clone()));
            }
            if matches!(server.status(), ServerStatus::Ended) {
                *end_timer += dt;
                if *end_timer >= END_HOLD {
                    let Some(m) = server.match_state() else {
                        return Some(OnlineOutcome::Left("MATCH ENDED".into()));
                    };
                    let winner =
                        m.winner
                            .unwrap_or_else(|| if m.score[0] >= m.score[1] { 0 } else { 1 });
                    let mut request = request.clone();
                    request.config = m.config;
                    request.local_side = 0;
                    return Some(OnlineOutcome::Finished(MatchReport {
                        request,
                        score: m.score,
                        winner,
                        stats: m.stats.clone(),
                        elapsed: m.elapsed,
                    }));
                }
            }
            None
        }
        OnlineSession::Client {
            client,
            request,
            char_id: _,
            end_timer,
            end_sound,
        } => {
            client.update(dt, input);
            for code in client.take_events() {
                play_code(audio, code);
            }
            let ended_state = client
                .latest()
                .map(|m| m.phase == Phase::Ended)
                .unwrap_or(false);
            if matches!(client.status(), ClientStatus::Disconnected(_)) && !ended_state {
                let ClientStatus::Disconnected(reason) = client.status() else {
                    unreachable!();
                };
                return Some(OnlineOutcome::Left(reason.clone()));
            }
            if ended_state {
                if !*end_sound {
                    *end_sound = true;
                    let local = client.side().unwrap_or(0);
                    let winner = client
                        .latest()
                        .and_then(|m| m.winner)
                        .unwrap_or(if local == 0 { 1 } else { 0 });
                    audio.play(if winner == local { Sfx::Win } else { Sfx::Lose });
                }
                *end_timer += dt;
                if *end_timer >= END_HOLD {
                    let Some(m) = client.latest() else {
                        return Some(OnlineOutcome::Left("NO MATCH DATA".into()));
                    };
                    let winner =
                        m.winner
                            .unwrap_or_else(|| if m.score[0] >= m.score[1] { 0 } else { 1 });
                    let mut request = request.clone();
                    request.config = m.config;
                    request.local_side = client.side().unwrap_or(0);
                    return Some(OnlineOutcome::Finished(MatchReport {
                        request,
                        score: m.score,
                        winner,
                        stats: m.stats.clone(),
                        elapsed: m.elapsed,
                    }));
                }
            } else {
                *end_timer = 0.0;
            }
            None
        }
    }
}

fn draw_session(
    canvas: &mut Canvas,
    assets: &Assets,
    save: &SaveData,
    audio: &Audio,
    session: &MatchSession,
) {
    let shake = if save.settings.shake {
        session.m.shake
    } else {
        0.0
    };
    canvas.begin_world(shake, session.m.time);
    render::draw_world(assets, &session.m);
    canvas.begin_ui();
    render::draw_match_ui(assets, &session.m, session.paused);
    if session.paused {
        render::draw_pause_overlay(assets);
        if audio.muted {
            render::draw_text_px(
                assets,
                "SOUND OFF",
                W / 2.0,
                268.0,
                0.34,
                render::rgba([255, 158, 158], 0.9),
                TextAlign::Center,
                false,
            );
        }
    }
    if save.settings.show_fps {
        render::draw_text_px(
            assets,
            &format!("{} FPS", get_fps()),
            W - 10.0,
            H - 18.0,
            0.24,
            render::rgba([240, 246, 255], 0.85),
            TextAlign::Right,
            true,
        );
    }
    if audio.muted && !session.paused {
        render::draw_text_px(
            assets,
            "MUTED",
            10.0,
            H - 18.0,
            0.24,
            render::rgba([255, 158, 158], 0.8),
            TextAlign::Left,
            true,
        );
    }
    canvas.finish();
}

fn draw_online_session(
    canvas: &mut Canvas,
    assets: &Assets,
    save: &SaveData,
    audio: &Audio,
    session: &OnlineSession,
    paused: bool,
) {
    match session {
        OnlineSession::Host { server, .. } => match server.status() {
            ServerStatus::Lobby => draw_host_lobby(canvas, assets, server),
            _ => {
                if let Some(m) = server.match_state() {
                    draw_online_match(canvas, assets, save, audio, m, paused, 0, true, 0.0, 0.0);
                } else {
                    draw_host_lobby(canvas, assets, server);
                }
            }
        },
        OnlineSession::Client {
            client, char_id, ..
        } => match client.status() {
            ClientStatus::Connecting | ClientStatus::Lobby => {
                draw_client_lobby(canvas, assets, client, *char_id);
            }
            _ => {
                if let Some(view) = client.view() {
                    draw_online_match(
                        canvas,
                        assets,
                        save,
                        audio,
                        &view,
                        paused,
                        client.side().unwrap_or(0),
                        false,
                        client.ping_ms(),
                        client.loss_percent(),
                    );
                } else {
                    draw_client_lobby(canvas, assets, client, *char_id);
                }
            }
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_online_match(
    canvas: &mut Canvas,
    assets: &Assets,
    save: &SaveData,
    audio: &Audio,
    m: &Match,
    paused: bool,
    local_side: usize,
    is_host: bool,
    ping: f32,
    loss: f32,
) {
    let shake = if save.settings.shake { m.shake } else { 0.0 };
    canvas.begin_world(shake, m.time);
    render::draw_world(assets, m);
    canvas.begin_ui();
    render::draw_match_ui(assets, m, paused);
    render::draw_text_px(
        assets,
        &format!("ONLINE  ·  {}", net::side_label(local_side)),
        10.0,
        H - 18.0,
        0.22,
        render::rgba([196, 230, 255], 0.9),
        TextAlign::Left,
        true,
    );
    let right = if is_host {
        "HOST  ·  BEST EFFORT UDP".to_string()
    } else {
        format!("PING {ping:.0} MS  ·  LOSS {loss:.0}%")
    };
    render::draw_text_px(
        assets,
        &right,
        W - 10.0,
        H - 18.0,
        0.22,
        render::rgba([174, 200, 222], 0.9),
        TextAlign::Right,
        true,
    );
    if paused {
        render::draw_pause_overlay(assets);
        render::draw_text_px(
            assets,
            "THE MATCH KEEPS RUNNING",
            W / 2.0,
            270.0,
            0.3,
            render::rgba([255, 214, 120], 0.95),
            TextAlign::Center,
            false,
        );
    }
    if save.settings.show_fps {
        render::draw_text_px(
            assets,
            &format!("{} FPS", get_fps()),
            W - 10.0,
            12.0,
            0.2,
            render::rgba([240, 246, 255], 0.7),
            TextAlign::Right,
            true,
        );
    }
    if audio.muted {
        render::draw_text_px(
            assets,
            "MUTED",
            10.0,
            12.0,
            0.2,
            render::rgba([255, 158, 158], 0.8),
            TextAlign::Left,
            true,
        );
    }
    canvas.finish();
}

fn draw_lobby_slime(
    assets: &Assets,
    index: usize,
    center_x: f32,
    bottom: f32,
    width: f32,
    flip: bool,
) {
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
    draw_ellipse(
        center_x,
        bottom,
        width * 0.45,
        width * 0.08,
        0.0,
        render::rgba([8, 12, 16], 0.3),
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_slot_card(
    assets: &Assets,
    x: f32,
    y: f32,
    side: usize,
    char_id: usize,
    title: &str,
    name: &str,
    status: &str,
    ready: bool,
    time: f32,
) {
    let w = 226.0;
    let h = 126.0;
    render::panel(
        x,
        y,
        w,
        h,
        render::rgba([12, 18, 29], 0.94),
        if ready {
            render::side_color(side)
        } else {
            render::rgba([48, 60, 80], 0.9)
        },
    );
    render::draw_text_px(
        assets,
        title,
        x + 10.0,
        y + 6.0,
        0.24,
        render::side_color(side),
        TextAlign::Left,
        false,
    );
    render::draw_text_px(
        assets,
        name,
        x + 10.0,
        y + 23.0,
        0.3,
        if ready {
            render::rgba([246, 250, 255], 1.0)
        } else {
            render::rgba([158, 174, 196], 0.95)
        },
        TextAlign::Left,
        false,
    );
    if ready {
        let bob = (time * 2.0 + side as f32).sin() * 2.0;
        draw_lobby_slime(
            assets,
            char_id,
            x + w / 2.0,
            y + h - 10.0 + bob,
            82.0,
            side == 1,
        );
    } else {
        render::draw_text_px(
            assets,
            "?",
            x + w / 2.0,
            y + h - 54.0,
            1.6,
            render::rgba([72, 86, 106], 0.9),
            TextAlign::Center,
            false,
        );
    }
    render::draw_text_px(
        assets,
        status,
        x + 10.0,
        y + 44.0,
        0.21,
        if ready {
            render::rgba([142, 230, 79], 0.95)
        } else {
            render::rgba([255, 212, 90], 0.95)
        },
        TextAlign::Left,
        false,
    );
}

fn draw_host_lobby(canvas: &mut Canvas, assets: &Assets, server: &Server) {
    let time = get_time() as f32;
    let cfg = server.config();
    canvas.begin_world(0.0, time);
    render::draw_menu_background(assets, cfg.arena, time);
    canvas.begin_ui();
    render::draw_text_px(
        assets,
        "HOSTING ONLINE MATCH",
        24.0,
        12.0,
        0.56,
        render::rgba([255, 246, 214], 1.0),
        TextAlign::Left,
        true,
    );
    render::draw_text_px(
        assets,
        "WAITING FOR A RIVAL...",
        W - 24.0,
        16.0,
        0.26,
        render::rgba([255, 212, 90], 0.95),
        TextAlign::Right,
        false,
    );

    render::panel(
        52.0,
        66.0,
        W - 104.0,
        64.0,
        render::rgba([10, 16, 26], 0.9),
        render::rgba([142, 230, 79], 0.9),
    );
    render::draw_text_px(
        assets,
        "SHARE THIS ADDRESS",
        64.0,
        74.0,
        0.22,
        render::rgba([150, 166, 188], 0.95),
        TextAlign::Left,
        false,
    );
    render::draw_text_px(
        assets,
        &server.local_address(),
        W / 2.0,
        92.0,
        0.46,
        render::rgba([236, 248, 255], 1.0),
        TextAlign::Center,
        false,
    );

    let sport = cfg.sport.label().to_uppercase();
    let arena = ARENAS[cfg.arena.min(ARENAS.len() - 1)].name.to_uppercase();
    render::draw_text_px(
        assets,
        &format!("{}  ·  {}  ·  FIRST TO {}", sport, arena, cfg.play_to),
        W / 2.0,
        140.0,
        0.24,
        render::rgba([178, 194, 214], 0.95),
        TextAlign::Center,
        false,
    );

    let peers = server.peers();
    let local_name = &CHARACTERS[cfg.p1.min(CHARACTERS.len() - 1)].name;
    render::draw_text_px(
        assets,
        "YOUR SLIME",
        40.0,
        162.0,
        0.2,
        render::rgba([150, 166, 188], 0.95),
        TextAlign::Left,
        false,
    );
    draw_slot_card(
        assets,
        40.0,
        178.0,
        0,
        cfg.p1,
        "PLAYER 1",
        &local_name.to_uppercase(),
        "READY",
        true,
        time,
    );
    match peers.first() {
        Some(peer) => {
            render::draw_text_px(
                assets,
                "RIVAL JOINED",
                292.0,
                162.0,
                0.2,
                render::rgba([150, 166, 188], 0.95),
                TextAlign::Left,
                false,
            );
            draw_slot_card(
                assets,
                292.0,
                178.0,
                1,
                peer.char_id,
                "PLAYER 2",
                &peer.name,
                "READY",
                true,
                time,
            );
        }
        None => {
            render::draw_text_px(
                assets,
                "RIVAL SLOT",
                292.0,
                162.0,
                0.2,
                render::rgba([150, 166, 188], 0.95),
                TextAlign::Left,
                false,
            );
            draw_slot_card(
                assets,
                292.0,
                178.0,
                1,
                cfg.p2,
                "PLAYER 2",
                "OPEN SLOT",
                "WAITING FOR A RIVAL...",
                false,
                time,
            );
        }
    }

    render::draw_text_px(
        assets,
        "LAN PLAYERS FIND YOU AUTOMATICALLY. INTERNET PLAYERS CAN TYPE YOUR ADDRESS.",
        W / 2.0,
        322.0,
        0.21,
        render::rgba([150, 166, 188], 0.95),
        TextAlign::Center,
        false,
    );
    render::draw_text_px(
        assets,
        "ESC CANCEL",
        W / 2.0,
        352.0,
        0.24,
        render::rgba([128, 142, 162], 0.9),
        TextAlign::Center,
        false,
    );

    canvas.finish();
}

fn draw_client_lobby(canvas: &mut Canvas, assets: &Assets, client: &Client, char_id: usize) {
    let time = get_time() as f32;
    canvas.begin_world(0.0, time);
    render::draw_menu_background(assets, 0, time);
    canvas.begin_ui();

    let connecting = matches!(client.status(), ClientStatus::Connecting);
    render::draw_text_px(
        assets,
        if connecting {
            "CONNECTING..."
        } else {
            "ONLINE LOBBY"
        },
        24.0,
        12.0,
        0.56,
        render::rgba([255, 246, 214], 1.0),
        TextAlign::Left,
        true,
    );
    render::draw_text_px(
        assets,
        "ESC CANCEL",
        W - 24.0,
        16.0,
        0.24,
        render::rgba([128, 142, 162], 0.9),
        TextAlign::Right,
        false,
    );

    render::panel(
        52.0,
        66.0,
        W - 104.0,
        64.0,
        render::rgba([10, 16, 26], 0.9),
        render::rgba([84, 160, 255], 0.9),
    );
    render::draw_text_px(
        assets,
        "SERVER",
        64.0,
        74.0,
        0.22,
        render::rgba([150, 166, 188], 0.95),
        TextAlign::Left,
        false,
    );
    render::draw_text_px(
        assets,
        &client.server_addr().to_string(),
        W / 2.0,
        92.0,
        0.42,
        render::rgba([236, 248, 255], 1.0),
        TextAlign::Center,
        false,
    );

    if connecting {
        let dots = ".".repeat((time * 2.0) as usize % 4);
        render::draw_text_px(
            assets,
            &format!("CONTACTING THE HOST{dots}"),
            W / 2.0,
            164.0,
            0.42,
            render::rgba([214, 228, 245], 0.95),
            TextAlign::Center,
            false,
        );
        render::draw_text_px(
            assets,
            "MAKE SURE THE HOST IS RUNNING AND UDP 43210 IS OPEN.",
            W / 2.0,
            192.0,
            0.23,
            render::rgba([150, 166, 188], 0.95),
            TextAlign::Center,
            false,
        );
        render::draw_text_px(
            assets,
            "CTRL+C / ESC TO GIVE UP",
            W / 2.0,
            352.0,
            0.22,
            render::rgba([128, 142, 162], 0.9),
            TextAlign::Center,
            false,
        );
        canvas.finish();
        return;
    }

    render::draw_text_px(
        assets,
        "WAITING FOR THE HOST TO START",
        W / 2.0,
        152.0,
        0.36,
        render::rgba([255, 226, 150], 0.95),
        TextAlign::Center,
        false,
    );
    render::draw_text_px(
        assets,
        &format!(
            "{} / {} PLAYERS  ·  {}",
            client.lobby_players(),
            client.lobby_needed(),
            client.server_name()
        ),
        W / 2.0,
        178.0,
        0.24,
        render::rgba([178, 194, 214], 0.95),
        TextAlign::Center,
        false,
    );

    let side = client.side().unwrap_or(0);
    render::draw_text_px(
        assets,
        &format!("YOU ARE {}", net::side_label(side)),
        W / 2.0,
        200.0,
        0.24,
        render::side_color(side),
        TextAlign::Center,
        false,
    );
    draw_slot_card(
        assets,
        W / 2.0 - 113.0,
        216.0,
        side,
        char_id,
        "YOUR SLIME",
        &CHARACTERS[char_id.min(CHARACTERS.len() - 1)]
            .name
            .to_uppercase(),
        "READY",
        true,
        time,
    );
    render::draw_text_px(
        assets,
        "THE MATCH STARTS AS SOON AS BOTH SLIMES ARE IN.",
        W / 2.0,
        356.0,
        0.21,
        render::rgba([150, 166, 188], 0.95),
        TextAlign::Center,
        false,
    );
    canvas.finish();
}

pub async fn run(cli: Cli) {
    let mut save = SaveData::load();
    i18n::set_language(save.settings.language);
    let assets = Assets::load();
    let mut audio = Audio::load(&save.settings).await;
    let mut canvas = Canvas::new();
    let mut front = FrontEnd::new(&save);
    let mut session: Option<MatchSession> = None;
    let mut online: Option<OnlineSession> = None;
    let mut online_paused = false;
    let mut fullscreen = false;

    if cli.host {
        match start_host(
            cli.match_config(net::now_seed(), 1.0),
            Difficulty::Normal,
            cli.port,
            &cli.name,
        ) {
            Ok(session) => online = Some(session),
            Err(error) => front.to_online_notice(error),
        }
    } else if let Some(addr) = cli.connect.clone() {
        match start_client(&addr, cli.p1, &cli.name) {
            Ok(session) => online = Some(session),
            Err(error) => front.to_online_notice(error),
        }
    }

    loop {
        let dt = get_frame_time().min(0.25);

        if is_key_pressed(KeyCode::F11) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }
        if (audio.music_volume - save.settings.music).abs() > 0.0001 {
            audio.set_music_volume(save.settings.music);
        }
        if (audio.sfx_volume - save.settings.sfx).abs() > 0.0001 {
            audio.set_sfx_volume(save.settings.sfx);
        }

        let mut finished: Option<MatchReport> = None;
        let mut abandon = false;

        if let Some(s) = session.as_mut() {
            if is_key_pressed(KeyCode::Escape) || (s.paused && is_key_pressed(KeyCode::Enter)) {
                s.paused = !s.paused;
                audio.play(Sfx::Ui);
            }
            if is_key_pressed(KeyCode::M) {
                audio.toggle_muted();
            }
            if s.paused && is_key_pressed(KeyCode::Q) {
                abandon = true;
            }

            if !s.paused && !abandon {
                if s.m.phase == Phase::Ended {
                    s.end_timer += dt;
                    if s.end_timer >= END_HOLD {
                        finished = Some(s.report());
                    }
                } else {
                    s.accumulator += dt;
                    let mut steps = 0;
                    while s.accumulator >= STEP && steps < MAX_STEPS {
                        let p1 = MatchSession::human_input(&save.settings.p1);
                        let p2 = match s.ai.as_mut() {
                            Some(ai) => ai.update(&s.m, STEP),
                            None => MatchSession::human_input(&save.settings.p2),
                        };
                        s.m.update(STEP, [p1, p2]);
                        s.accumulator -= STEP;
                        steps += 1;
                    }
                    if steps == MAX_STEPS {
                        s.accumulator = 0.0;
                    }
                }
            }

            let events = s.m.take_events();
            for event in &events {
                play_event(&audio, event, s.request.ai, None);
            }
        }

        if abandon {
            session = None;
            front.championship = None;
            front.to_title();
            audio.play(Sfx::Ui);
        }

        let mut online_outcome: Option<OnlineOutcome> = None;
        if let Some(current) = online.as_mut() {
            let in_match = match current {
                OnlineSession::Host { server, .. } => {
                    matches!(server.status(), ServerStatus::Playing | ServerStatus::Ended)
                }
                OnlineSession::Client { client, .. } => {
                    matches!(client.status(), ClientStatus::Playing | ClientStatus::Ended)
                }
            };
            if in_match {
                if is_key_pressed(KeyCode::Escape) {
                    online_paused = !online_paused;
                    audio.play(Sfx::Ui);
                }
                if is_key_pressed(KeyCode::M) {
                    audio.toggle_muted();
                }
                if online_paused && is_key_pressed(KeyCode::Q) {
                    online_outcome = Some(OnlineOutcome::Left("LEFT MATCH".into()));
                }
            } else if is_key_pressed(KeyCode::Escape) {
                online_outcome = Some(OnlineOutcome::Left("LEFT LOBBY".into()));
            }
            if online_outcome.is_none() {
                let input = MatchSession::human_input(&save.settings.p1);
                online_outcome = update_online(current, dt, input, online_paused, &audio);
            }
        }

        if let Some(outcome) = online_outcome {
            online = None;
            online_paused = false;
            match outcome {
                OnlineOutcome::Finished(report) => {
                    front.on_match_finished(report, &mut save);
                    let _ = save.write();
                    front.dirty = false;
                }
                OnlineOutcome::Left(reason) => {
                    front.to_online_notice(format!("ONLINE: {reason}"));
                    audio.play(Sfx::Ui);
                }
            }
        }

        if let Some(report) = finished {
            front.on_match_finished(report, &mut save);
            let _ = save.write();
            front.dirty = false;
            session = None;
        }

        let mut drew_match = false;
        if let Some(s) = session.as_ref() {
            draw_session(&mut canvas, &assets, &save, &audio, s);
            drew_match = true;
        } else if let Some(current) = online.as_ref() {
            draw_online_session(&mut canvas, &assets, &save, &audio, current, online_paused);
            drew_match = true;
        }

        if !drew_match {
            if let Some(action) = front.frame(&mut canvas, &assets, &mut save, &audio) {
                match action {
                    UiAction::Launch(request) => session = Some(MatchSession::new(request)),
                    UiAction::HostOnline(request) => {
                        match start_host(
                            request.config,
                            request.difficulty,
                            DEFAULT_PORT,
                            "SUPER SLIME",
                        ) {
                            Ok(hosted) => {
                                online = Some(hosted);
                                online_paused = false;
                                session = None;
                            }
                            Err(error) => front.to_online_notice(error),
                        }
                    }
                    UiAction::JoinOnline { addr, char_id } => {
                        match start_client(&addr, char_id, "CHALLENGER") {
                            Ok(joined) => {
                                online = Some(joined);
                                online_paused = false;
                                session = None;
                            }
                            Err(error) => front.to_online_notice(error),
                        }
                    }
                    UiAction::Quit => {
                        let _ = save.write();
                        break;
                    }
                }
                front.dirty = false;
            }
        }

        if front.dirty {
            let _ = save.write();
            front.dirty = false;
        }

        next_frame().await;
    }
}
