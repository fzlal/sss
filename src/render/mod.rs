use macroquad::prelude::*;

use crate::assets::Assets;
use crate::game::arenas::{Ambient, Arena, ARENAS};
use crate::game::characters::{BuildKind, ProjKind};
use crate::game::config::*;
use crate::game::fx::ParticleShape;
use crate::game::sim::{Entity, EntityKind, Match, Phase};
use crate::i18n;

pub const FONT_CELL_W: f32 = 24.0;
pub const FONT_CELL_H: f32 = 36.0;
pub const FONT_ADVANCE: f32 = 20.0;

const BASELINE_RATIO: f32 = 0.75;
const PERSIAN_LINE_FACTOR: f32 = 1.22;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

pub fn rgba(color: [u8; 3], alpha: f32) -> Color {
    Color::new(
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
        alpha,
    )
}

pub fn side_color(side: usize) -> Color {
    if side == 0 {
        rgba([142, 230, 79], 1.0)
    } else {
        rgba([255, 143, 199], 1.0)
    }
}

pub struct Canvas {
    target: RenderTarget,
    ui_camera: Camera2D,
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl Canvas {
    pub fn new() -> Self {
        let target = render_target(W as u32, H as u32);
        target.texture.set_filter(FilterMode::Nearest);
        let mut ui_camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, W, H));
        ui_camera.render_target = Some(target.clone());
        Self { target, ui_camera }
    }

    fn viewport() -> (f32, f32, f32) {
        let raw = (screen_width() / W).min(screen_height() / H);
        let scale = if raw >= 1.0 {
            raw.floor()
        } else {
            raw.max(0.05)
        };
        let ox = (screen_width() - W * scale) * 0.5;
        let oy = (screen_height() - H * scale) * 0.5;
        (scale, ox, oy)
    }

    pub fn mouse(&self) -> Vec2 {
        let (scale, ox, oy) = Self::viewport();
        let (mx, my) = mouse_position();
        vec2((mx - ox) / scale, (my - oy) / scale)
    }

    pub fn mouse_inside(&self) -> bool {
        let m = self.mouse();
        m.x >= 0.0 && m.y >= 0.0 && m.x <= W && m.y <= H
    }

    pub fn begin_world(&mut self, shake: f32, time: f32) {
        let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, W, H));
        camera.render_target = Some(self.target.clone());
        if shake > 0.01 {
            let dx = (time * 161.0).sin() * shake;
            let dy = (time * 227.0 + 1.7).cos() * shake * 0.55;
            camera.target = vec2(W / 2.0 + dx, H / 2.0 + dy);
        }
        set_camera(&camera);
        clear_background(rgba([6, 8, 14], 1.0));
    }

    pub fn begin_ui(&self) {
        set_camera(&self.ui_camera);
    }

    pub fn finish(&self) {
        set_default_camera();
        clear_background(rgba([5, 6, 10], 1.0));
        let (scale, ox, oy) = Self::viewport();
        draw_texture_ex(
            &self.target.texture,
            ox,
            oy,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(W * scale, H * scale)),
                flip_y: true,
                ..Default::default()
            },
        );
    }
}

pub fn text_height(text: &str, scale: f32) -> f32 {
    let lines = text.lines().count().max(1) as f32;
    if i18n::is_persian() {
        FONT_CELL_H * scale + 4.0 * scale + (lines - 1.0) * persian_line_step(scale)
    } else {
        lines * (FONT_CELL_H * scale + 4.0 * scale)
    }
}

fn glyph_index(ch: char) -> Option<u32> {
    match ch {
        ' ' => None,
        // The source font strip starts at '!' and ends at 'z' (90 cells),
        // followed by a blank cell and a few unused symbols.
        '!'..='z' => Some(ch as u32 - 33),
        _ => Some('?' as u32 - 33),
    }
}

fn glyph(assets: &Assets, ch: char, x: f32, y: f32, scale: f32, color: Color) {
    if ch == '·' {
        let size = (3.0 * scale).max(1.0);
        draw_rectangle(x + 10.0 * scale, y + 18.0 * scale, size, size, color);
        return;
    }
    let Some(index) = glyph_index(ch) else {
        return;
    };
    let col = index as f32;
    draw_texture_ex(
        &assets.font,
        x,
        y,
        color,
        DrawTextureParams {
            dest_size: Some(vec2(FONT_CELL_W * scale, FONT_CELL_H * scale)),
            source: Some(Rect::new(col * FONT_CELL_W, 0.0, FONT_CELL_W, FONT_CELL_H)),
            ..Default::default()
        },
    );
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text_px(
    assets: &Assets,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    color: Color,
    align: TextAlign,
    shadow: bool,
) {
    if i18n::is_persian() {
        draw_text_fa(assets, text, x, y, scale, color, align, shadow);
        return;
    }
    let line_step = FONT_CELL_H * scale + 4.0 * scale;
    for (line_index, line) in text.lines().enumerate() {
        let line_width = line.chars().count() as f32 * FONT_ADVANCE * scale;
        let left = match align {
            TextAlign::Left => x,
            TextAlign::Center => x - line_width / 2.0,
            TextAlign::Right => x - line_width,
        };
        let ly = y + line_index as f32 * line_step;
        let mut pen = 0.0;
        for ch in line.chars() {
            if shadow {
                glyph(
                    assets,
                    ch,
                    left + pen + scale,
                    ly + scale * 1.5,
                    scale,
                    rgba([0, 0, 0], 0.55),
                );
            }
            glyph(assets, ch, left + pen, ly, scale, color);
            pen += FONT_ADVANCE * scale;
        }
    }
}

fn persian_font_size(scale: f32) -> u16 {
    (FONT_CELL_H * scale).round().clamp(1.0, 255.0) as u16
}

fn persian_line_step(scale: f32) -> f32 {
    FONT_CELL_H * scale * PERSIAN_LINE_FACTOR
}

fn measure_display(assets: &Assets, display: &str, scale: f32) -> f32 {
    measure_text(
        display,
        Some(&assets.font_fa),
        persian_font_size(scale),
        1.0,
    )
    .width
}

#[allow(clippy::too_many_arguments)]
fn draw_text_fa(
    assets: &Assets,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    color: Color,
    align: TextAlign,
    shadow: bool,
) {
    let localized = i18n::localize(text);
    let lines: Vec<String> = localized.lines().map(i18n::display_line).collect();
    let widths: Vec<f32> = lines
        .iter()
        .map(|line| measure_display(assets, line, scale))
        .collect();
    let font_size = persian_font_size(scale);
    let step = persian_line_step(scale);
    let first_baseline = y + FONT_CELL_H * scale * BASELINE_RATIO;
    for (index, line) in lines.iter().enumerate() {
        let width = widths[index];
        let left = match align {
            TextAlign::Left => x,
            TextAlign::Center => x - width / 2.0,
            TextAlign::Right => x - width,
        };
        let baseline = first_baseline + index as f32 * step;
        if shadow {
            draw_text_ex(
                line,
                left + scale,
                baseline + scale * 1.5,
                TextParams {
                    font: Some(&assets.font_fa),
                    font_size,
                    color: rgba([0, 0, 0], 0.55),
                    ..Default::default()
                },
            );
        }
        draw_text_ex(
            line,
            left,
            baseline,
            TextParams {
                font: Some(&assets.font_fa),
                font_size,
                color,
                ..Default::default()
            },
        );
    }
}

pub fn draw_text_shadow(
    assets: &Assets,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    color: Color,
    align: TextAlign,
) {
    draw_text_px(assets, text, x, y, scale, color, align, true);
}

pub fn wrap_text(assets: &Assets, text: &str, max_chars: usize, scale: f32) -> String {
    if !i18n::is_persian() {
        return wrap_text_chars(text, max_chars);
    }
    let localized = i18n::localize(text);
    let budget = max_chars as f32 * FONT_ADVANCE * scale;
    let font_size = persian_font_size(scale);
    let mut out = String::new();
    let mut line = String::new();
    for word in localized.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        let display = i18n::display_line(&candidate);
        let width = measure_text(&display, Some(&assets.font_fa), font_size, 1.0).width;
        if !line.is_empty() && width > budget {
            out.push_str(&line);
            out.push('\n');
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push_str(&line);
    }
    out
}

fn wrap_text_chars(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut line_len = 0usize;
    for word in text.split_whitespace() {
        let extra = if line_len == 0 {
            word.len()
        } else {
            word.len() + 1
        };
        if line_len + extra > max_chars && line_len > 0 {
            out.push('\n');
            line_len = 0;
        }
        if line_len > 0 {
            out.push(' ');
            line_len += 1;
        }
        out.push_str(word);
        line_len += word.len();
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn draw_tex(
    texture: &Texture2D,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    flip_x: bool,
    rotation: f32,
) {
    draw_texture_ex(
        texture,
        x,
        y,
        color,
        DrawTextureParams {
            dest_size: Some(vec2(w, h)),
            flip_x,
            rotation,
            ..Default::default()
        },
    );
}

fn hash(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9e37_79b1) ^ 0x85eb_ca6b;
    x ^= x >> 15;
    x = x.wrapping_mul(0xc2b2_ae35);
    x ^= x >> 13;
    (x & 0xffff) as f32 / 65535.0
}

pub fn panel(x: f32, y: f32, w: f32, h: f32, fill: Color, border: Color) {
    draw_rectangle(x, y, w, h, fill);
    draw_rectangle_lines(x, y, w, h, 2.0, border);
}

pub fn draw_ambient(arena: &Arena, time: f32) {
    match arena.ambient {
        Ambient::Leaves => {
            let colors = [
                rgba([255, 186, 86], 0.8),
                rgba([255, 126, 91], 0.75),
                rgba([170, 224, 110], 0.7),
            ];
            for i in 0..18u32 {
                let speed = 9.0 + hash(i + 4) * 18.0;
                let x = ((hash(i) * (W + 60.0) + time * speed) % (W + 60.0)) - 30.0;
                let y = ((hash(i + 40) * (GROUND_Y - 80.0) + time * (7.0 + hash(i + 9) * 13.0))
                    % (GROUND_Y - 80.0))
                    + 70.0
                    + (time * 1.7 + i as f32).sin() * 8.0;
                let w = 3.0 + hash(i + 12) * 3.5;
                draw_rectangle_ex(
                    x,
                    y,
                    w,
                    w * 0.55,
                    DrawRectangleParams {
                        offset: vec2(0.5, 0.5),
                        rotation: (time * 1.4 + i as f32).sin() * 1.2,
                        color: colors[(i as usize) % colors.len()],
                    },
                );
            }
        }
        Ambient::Dust => {
            for i in 0..26u32 {
                let x = (hash(i) * W + time * (4.0 + hash(i + 7) * 5.0)).rem_euclid(W);
                let y = 84.0 + hash(i + 30) * (GROUND_Y - 110.0);
                let alpha = 0.15 + 0.2 * (time * 1.3 + i as f32).sin().abs();
                draw_circle(x, y, 1.0 + hash(i + 5) * 1.3, rgba([255, 244, 214], alpha));
            }
        }
        Ambient::Sand => {
            for i in 0..24u32 {
                let x =
                    (hash(i) * W + time * (28.0 + hash(i + 3) * 24.0)).rem_euclid(W + 40.0) - 20.0;
                let y = 100.0 + hash(i + 18) * (GROUND_Y - 130.0);
                draw_line(x, y, x - 7.0, y + 1.0, 1.0, rgba([255, 220, 150], 0.22));
            }
        }
        Ambient::Bubbles => {
            for i in 0..18u32 {
                let speed = 8.0 + hash(i + 2) * 16.0;
                let x = hash(i) * W + (time * 0.6 + i as f32).sin() * 10.0;
                let y = GROUND_Y - ((hash(i + 11) * GROUND_Y + time * speed) % GROUND_Y);
                let r = 1.5 + hash(i + 6) * 3.0;
                draw_circle_lines(x, y, r, 1.0, rgba([200, 250, 255], 0.3));
            }
        }
        Ambient::Stars => {
            for i in 0..34u32 {
                let x = hash(i) * W;
                let y = 68.0 + hash(i + 50) * 240.0;
                let glow = ((time * 1.8 + i as f32 * 1.3).sin() * 0.5 + 0.5).powi(2);
                let size = 0.8 + glow * 1.4;
                draw_rectangle(x, y, size, size, rgba([255, 250, 214], 0.25 + glow * 0.6));
            }
        }
        Ambient::Sparkle => {
            let colors = [
                rgba([255, 222, 89], 0.75),
                rgba([255, 93, 241], 0.7),
                rgba([75, 229, 255], 0.7),
            ];
            for i in 0..22u32 {
                let x = hash(i) * W;
                let y = 70.0 + hash(i + 17) * (GROUND_Y - 90.0);
                let twinkle = ((time * 3.2 + i as f32 * 2.1).sin() * 0.5 + 0.5).powi(3);
                if twinkle < 0.12 {
                    continue;
                }
                let c = colors[(i as usize) % colors.len()];
                let s = 1.5 + twinkle * 2.0;
                draw_rectangle(x, y - s * 1.6, 1.4, s * 3.2, c);
                draw_rectangle(x - s * 1.6, y, s * 3.2, 1.4, c);
            }
        }
        Ambient::Rain => {
            for i in 0..30u32 {
                let speed = 190.0 + hash(i + 8) * 130.0;
                let x = (hash(i) * (W + 80.0) + time * 34.0).rem_euclid(W + 80.0) - 40.0;
                let y = ((hash(i + 21) * GROUND_Y + time * speed) % (GROUND_Y + 20.0)) + 40.0;
                draw_line(x, y, x - 3.0, y + 9.0, 1.0, rgba([185, 224, 255], 0.24));
            }
        }
        Ambient::Snow => {
            for i in 0..30u32 {
                let speed = 12.0 + hash(i + 2) * 20.0;
                let x = hash(i) * W + (time * 0.9 + i as f32).sin() * 14.0;
                let y = ((hash(i + 44) * GROUND_Y + time * speed) % (GROUND_Y + 10.0)) - 10.0;
                let r = 0.8 + hash(i + 7) * 1.4;
                draw_circle(x, y, r, rgba([255, 255, 255], 0.65));
            }
        }
    }
}

fn draw_soccer_goals(assets: &Assets, front: bool) {
    let texture = if front {
        &assets.goal_front
    } else {
        &assets.goal
    };
    let left_x = -16.0;
    let right_x = W - GOAL_DEPTH + 16.0;
    let y = GROUND_Y - 115.0;
    draw_tex(texture, left_x, y, 70.0, 115.0, WHITE, false, 0.0);
    draw_tex(texture, right_x - 70.0, y, 70.0, 115.0, WHITE, true, 0.0);
}

fn draw_baskets(assets: &Assets) {
    let y = HOOP_Y - 36.0;
    draw_tex(
        &assets.basket,
        HOOP_LEFT_X - 39.0,
        y,
        78.0,
        80.0,
        WHITE,
        false,
        0.0,
    );
    draw_tex(
        &assets.basket,
        HOOP_RIGHT_X - 39.0,
        y,
        78.0,
        80.0,
        WHITE,
        true,
        0.0,
    );
}

fn draw_net(assets: &Assets) {
    draw_tex(
        &assets.net,
        NET_X - 7.0,
        GROUND_Y - 92.0,
        14.0,
        92.0,
        WHITE,
        false,
        0.0,
    );
    draw_rectangle(
        NET_X - 1.0,
        GROUND_Y - 92.0,
        2.0,
        92.0,
        rgba([20, 24, 30], 0.25),
    );
}

fn draw_player(assets: &Assets, m: &Match, index: usize) {
    let p = &m.players[index];
    let texture = assets.slime_for(p.char_id);
    let sprite_w = 56.0 * p.scale;
    let sprite_h = texture.height() * (sprite_w / texture.width());
    let bob = if p.on_ground && p.vx.abs() > 30.0 {
        (p.step * 15.0).sin().abs() * 1.6
    } else {
        0.0
    };
    let bottom = p.bottom() + 2.0 - bob;
    let mut tint = WHITE;
    if p.buffs.chill > 0.0 {
        tint = rgba([180, 235, 255], 1.0);
    } else if p.buffs.rage > 0.0 {
        tint = rgba([255, 210, 190], 1.0);
    } else if p.buffs.sprint > 0.0 {
        tint = rgba([255, 250, 205], 1.0);
    }
    let alpha = if p.buffs.invisible > 0.0 { 0.28 } else { 1.0 };
    let (r, g, b, _) = (tint.r, tint.g, tint.b, tint.a);
    tint = Color::new(r, g, b, alpha);

    let shadow_scale = p.scale.max(0.7);
    draw_ellipse(
        p.x,
        GROUND_Y - 3.0,
        30.0 * shadow_scale,
        6.0 * shadow_scale,
        0.0,
        rgba([10, 16, 20], 0.25),
    );

    if p.buffs.rage > 0.0 {
        let pulse = 1.0 + (m.time * 9.0).sin() * 0.08;
        draw_ellipse(
            p.x,
            bottom - 6.0,
            34.0 * pulse,
            20.0 * pulse,
            0.0,
            rgba([255, 83, 61], 0.16),
        );
    }
    if p.buffs.dunk > 0.0 {
        draw_ellipse(
            p.x,
            GROUND_Y - 2.0,
            30.0,
            8.0,
            0.0,
            rgba([255, 210, 74], 0.4),
        );
    }

    let x = p.x - sprite_w / 2.0;
    let y = bottom - sprite_h;
    let flip = p.facing < 0.0;
    draw_tex(texture, x, y, sprite_w, sprite_h, tint, flip, 0.0);
    if p.buffs.chill > 0.0 {
        let shiver = (m.time * 30.0).sin() * 1.2;
        draw_circle_lines(
            x + sprite_w / 2.0 + shiver,
            y + sprite_h * 0.55,
            sprite_w * 0.36,
            1.0,
            rgba([210, 250, 255], 0.55),
        );
    }
    if p.frozen {
        draw_rectangle(
            x - 2.0 + 1.0,
            y - 2.0,
            sprite_w + 4.0,
            sprite_h + 4.0,
            rgba([160, 230, 255], 0.34),
        );
        draw_rectangle_lines(
            x - 2.0,
            y - 2.0,
            sprite_w + 4.0,
            sprite_h + 4.0,
            2.0,
            rgba([220, 250, 255], 0.75),
        );
        for i in 0..4 {
            let fx = x + ((i as f32 * 17.0 + 7.0) % sprite_w.max(1.0));
            let fy = y + ((i as f32 * 13.0 + 5.0) % sprite_h.max(1.0));
            draw_line(fx - 3.0, fy, fx + 3.0, fy, 1.0, rgba([240, 255, 255], 0.7));
            draw_line(fx, fy - 3.0, fx, fy + 3.0, 1.0, rgba([240, 255, 255], 0.7));
        }
    }
    if p.pull_timer > 0.0 || p.remote_timer > 0.0 {
        let color = if p.pull_timer > 0.0 {
            side_color(index)
        } else {
            rgba([75, 229, 255], 0.8)
        };
        let dashed = p.remote_timer > 0.0;
        draw_energy_beam(p.x, p.y - 6.0, m.ball.x, m.ball.y, color, dashed, m.time);
    }
    if p.drain_timer > 0.0 {
        let opp = &m.players[1 - index];
        draw_energy_beam(
            p.x,
            p.y - 8.0,
            opp.x,
            opp.y,
            rgba([255, 120, 190], 0.7),
            true,
            m.time,
        );
    }
    if p.frozen {
        draw_text_px(
            assets,
            "FROZEN",
            p.x,
            p.y - p.h / 2.0 - 20.0,
            0.4,
            rgba([200, 245, 255], 0.9),
            TextAlign::Center,
            true,
        );
    }
}

fn draw_energy_beam(x1: f32, y1: f32, x2: f32, y2: f32, color: Color, dashed: bool, time: f32) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = dx.hypot(dy).max(1.0);
    let nx = dx / len;
    let ny = dy / len;
    let segments = (len / 7.0).ceil().max(1.0) as usize;
    for i in 0..segments {
        if dashed && i % 2 == 1 {
            continue;
        }
        let t0 = i as f32 / segments as f32;
        let t1 = (i as f32 + 0.7) / segments as f32;
        let wobble = (time * 18.0 + i as f32).sin() * 1.4;
        draw_line(
            x1 + dx * t0 - ny * wobble,
            y1 + dy * t0 + nx * wobble,
            x1 + dx * t1 - ny * wobble,
            y1 + dy * t1 + nx * wobble,
            if dashed { 1.0 } else { 2.0 },
            Color::new(color.r, color.g, color.b, color.a * 0.6),
        );
    }
}

fn draw_ball(assets: &Assets, m: &Match) {
    let b = &m.ball;
    let height = (GROUND_Y - b.y).max(0.0);
    let shadow_w = (2.0 * b.r + 7.0) * (1.0 - (height / 340.0).min(0.6));
    draw_ellipse(
        b.x,
        GROUND_Y - 2.0,
        shadow_w,
        shadow_w * 0.22,
        0.0,
        rgba([8, 12, 16], 0.28),
    );
    if b.y + b.r < CEILING_Y {
        return;
    }
    let texture = assets.ball_for(m.sport);
    let size = b.r * 2.0;
    let mut color = WHITE;
    if b.chill_timer > 0.0 {
        color = rgba([175, 232, 255], 1.0);
    }
    draw_texture_ex(
        texture,
        b.x - size / 2.0,
        b.y - size / 2.0,
        color,
        DrawTextureParams {
            dest_size: Some(vec2(size, size)),
            rotation: b.spin,
            ..Default::default()
        },
    );
    if b.chill_timer > 0.0 {
        for i in 0..3 {
            let a = m.time * 2.3 + i as f32 * 2.09;
            draw_circle(
                b.x + a.cos() * (b.r + 2.0),
                b.y + a.sin() * (b.r + 2.0),
                1.2,
                rgba([220, 250, 255], 0.75),
            );
        }
    }
}

fn draw_solid(assets: &Assets, e: &Entity, kind: BuildKind, alpha: f32) {
    let x = e.x - e.w / 2.0;
    let top = e.y - e.h;
    match kind {
        BuildKind::Wall => {
            let mut y = top;
            while y < e.y - 0.1 {
                let h = (y + 16.0).min(e.y) - y;
                let flip = ((y - top) / 16.0) as i32 % 2 == 1;
                draw_tex(
                    &assets.brick,
                    x,
                    y,
                    e.w,
                    h,
                    rgba([255, 255, 255], alpha),
                    flip,
                    0.0,
                );
                y += 16.0;
            }
        }
        BuildKind::Tree => {
            let h = 56.0;
            let w = h * (94.0 / 117.0);
            draw_tex(
                &assets.tree,
                e.x - w / 2.0,
                e.y - h,
                w,
                h,
                rgba([255, 255, 255], alpha),
                false,
                0.0,
            );
        }
        BuildKind::Signs => {
            draw_rectangle(e.x - 1.0, e.y - 19.0, 2.0, 19.0, rgba([80, 84, 92], alpha));
            let size = 25.0;
            draw_tex(
                &assets.stop_sign,
                e.x - size / 2.0,
                e.y - 19.0 - size * 0.72,
                size,
                size,
                rgba([255, 255, 255], alpha),
                false,
                0.0,
            );
        }
    }
}

fn projectile_texture(assets: &Assets, kind: ProjKind) -> Option<&Texture2D> {
    match kind {
        ProjKind::Arrow => Some(&assets.arrow),
        ProjKind::Boomerang => Some(&assets.boomerang),
        ProjKind::Glove => Some(&assets.glove),
        ProjKind::Harpoon => Some(&assets.harpoon),
        ProjKind::Hook => Some(&assets.hook),
        ProjKind::Fireball => Some(&assets.orb),
        ProjKind::Bullet | ProjKind::Bomb => None,
    }
}

fn draw_entity(assets: &Assets, m: &Match, e: &Entity) {
    let fade = (e.life / 0.25).clamp(0.0, 1.0);
    match &e.kind {
        EntityKind::Projectile {
            kind,
            r,
            age,
            returning,
            ..
        } => {
            let rot = if *kind == ProjKind::Boomerang {
                age * 18.0 * if *returning { -1.0 } else { 1.0 }
            } else {
                e.vy.atan2(e.vx)
            };
            if let Some(texture) = projectile_texture(assets, *kind) {
                let (mut w, mut h) = match kind {
                    ProjKind::Arrow => (30.0, 7.5),
                    ProjKind::Boomerang => (28.0, 10.0),
                    ProjKind::Glove => (24.0, 13.0),
                    ProjKind::Harpoon => (28.0, 6.0),
                    ProjKind::Hook => (24.0, 20.0),
                    ProjKind::Fireball => (24.0, 24.0),
                    _ => (r * 2.0, r * 2.0),
                };
                if *returning {
                    w *= 0.9;
                    h *= 0.9;
                }
                if *kind == ProjKind::Arrow && e.vx < 0.0 {
                    draw_tex(
                        texture,
                        e.x - w / 2.0,
                        e.y - h / 2.0,
                        w,
                        h,
                        WHITE,
                        false,
                        std::f32::consts::PI,
                    );
                } else {
                    let flip = matches!(kind, ProjKind::Arrow | ProjKind::Harpoon) && e.vx < 0.0;
                    draw_texture_ex(
                        texture,
                        e.x - w / 2.0,
                        e.y - h / 2.0,
                        if matches!(kind, ProjKind::Fireball) {
                            rgba([255, 160, 70], 1.0)
                        } else {
                            WHITE
                        },
                        DrawTextureParams {
                            dest_size: Some(vec2(w, h)),
                            rotation: rot,
                            flip_x: flip,
                            ..Default::default()
                        },
                    );
                }
                if matches!(kind, ProjKind::Fireball) {
                    draw_circle(
                        e.x,
                        e.y,
                        r + 2.0 + (m.time * 14.0).sin() * 1.5,
                        rgba([255, 130, 40], 0.18),
                    );
                }
            } else if *kind == ProjKind::Bomb {
                draw_circle(e.x, e.y, *r, rgba([36, 34, 44], 1.0));
                draw_circle_lines(e.x, e.y, *r, 1.0, rgba([255, 201, 61], 0.8));
                draw_line(
                    e.x + 2.0,
                    e.y - *r,
                    e.x + 5.0,
                    e.y - *r - 5.0,
                    1.5,
                    rgba([255, 230, 160], 1.0),
                );
                let blink = ((m.time * 22.0).sin() * 0.5 + 0.5).powi(4);
                draw_circle(
                    e.x + 5.0,
                    e.y - *r - 5.0,
                    1.5 + blink * 1.5,
                    rgba([255, 220, 90], 0.9),
                );
            } else {
                let dir = e.vy.atan2(e.vx);
                draw_line(
                    e.x - dir.cos() * 12.0,
                    e.y - dir.sin() * 12.0,
                    e.x,
                    e.y,
                    2.0,
                    rgba([255, 244, 160], 0.9),
                );
                draw_circle(e.x, e.y, 2.2, rgba([255, 255, 230], 1.0));
            }
        }
        EntityKind::Solid { kind, .. } => draw_solid(assets, e, *kind, fade),
        EntityKind::Wave {
            radius,
            max_radius,
            knock,
            applied_ball,
            ..
        } => {
            let alpha = (1.0 - radius / max_radius.max(1.0)).clamp(0.0, 1.0);
            let color = rgba([255, 140, 220], 0.55 * alpha + 0.1);
            draw_circle_lines(e.x, e.y, *radius, 3.0, color);
            draw_circle_lines(
                e.x,
                e.y,
                *radius * 0.86,
                1.0,
                rgba([255, 255, 255], 0.2 * alpha),
            );
            if *applied_ball && *knock > 300.0 && alpha > 0.4 {
                draw_circle(e.x, e.y, 2.0, rgba([255, 255, 255], alpha));
            }
        }
        EntityKind::Geyser {
            warning,
            duration,
            erupted,
        } => {
            let total = warning + duration;
            let elapsed = total - e.life;
            if !*erupted {
                let pulse = ((m.time * 12.0).sin() * 0.5 + 0.5).powi(2);
                draw_ellipse(
                    e.x,
                    GROUND_Y - 3.0,
                    24.0,
                    6.0,
                    0.0,
                    rgba([120, 230, 255], 0.18 + pulse * 0.3),
                );
                draw_circle_lines(
                    e.x,
                    GROUND_Y - 4.0,
                    9.0 + pulse * 4.0,
                    1.5,
                    rgba([190, 250, 255], 0.7),
                );
            } else {
                let er = (elapsed - warning).max(0.0);
                let height = (er * 380.0).min(130.0);
                let fade_out = (e.life / 0.4).clamp(0.0, 1.0);
                for i in 0..7 {
                    let t = i as f32 / 7.0;
                    let y = GROUND_Y - height * t;
                    let r = 10.0 - t * 7.0;
                    draw_circle(
                        e.x + (m.time * 20.0 + i as f32 * 1.7).sin() * (2.0 + t * 5.0),
                        y,
                        r,
                        rgba([175, 243, 255], (0.55 - t * 0.3) * fade_out),
                    );
                }
                draw_line(
                    e.x,
                    GROUND_Y,
                    e.x,
                    GROUND_Y - height,
                    2.0,
                    rgba([230, 255, 255], 0.5 * fade_out),
                );
            }
        }
        EntityKind::Zone { radius } => {
            let mud = rgba([92, 62, 38], 0.64);
            draw_ellipse(e.x, GROUND_Y - 3.0, radius * 1.85, 12.0, 0.0, mud);
            draw_ellipse(
                e.x - 8.0,
                GROUND_Y - 5.0,
                radius * 0.85,
                5.0,
                0.0,
                rgba([120, 84, 49], 0.7),
            );
            draw_ellipse(
                e.x + 10.0,
                GROUND_Y - 3.5,
                radius * 0.65,
                4.0,
                0.0,
                rgba([67, 45, 28], 0.65),
            );
            let bob = (m.time * 3.0).sin() * 1.5;
            draw_circle(
                e.x + radius * 0.4,
                GROUND_Y - 8.0 + bob,
                1.3,
                rgba([180, 150, 100], 0.6),
            );
        }
        EntityKind::BlackHole { radius, .. } => {
            let pulse = 1.0 + (m.time * 4.0).sin() * 0.05;
            draw_circle(e.x, e.y, radius * 1.8 * pulse, rgba([73, 36, 120], 0.16));
            draw_circle_lines(
                e.x,
                e.y,
                radius * 1.45 * pulse,
                2.0,
                rgba([172, 104, 255], 0.5),
            );
            draw_circle(e.x, e.y, *radius, rgba([8, 4, 16], 0.96));
            draw_circle_lines(e.x, e.y, *radius, 2.0, rgba([210, 160, 255], 0.65));
            for i in 0..3 {
                let a = m.time * (2.4 + i as f32 * 0.7) + i as f32 * 2.1;
                let rr = radius * (1.2 + i as f32 * 0.28);
                draw_circle(
                    e.x + a.cos() * rr,
                    e.y + a.sin() * rr * 0.8,
                    1.4,
                    rgba([255, 255, 255], 0.55),
                );
            }
        }
        EntityKind::Disco {
            radius,
            orb_r,
            angle,
            ..
        } => {
            let owner = &m.players[e.owner];
            let ox = owner.x;
            let oy = owner.y - 8.0;
            let colors = [
                rgba([255, 217, 74], 0.5),
                rgba([255, 93, 241], 0.5),
                rgba([75, 229, 255], 0.5),
            ];
            for i in 0..6 {
                let a = *angle + i as f32 * std::f32::consts::TAU / 6.0;
                draw_line(
                    ox,
                    oy,
                    ox + a.cos() * radius,
                    oy + a.sin() * 16.0,
                    1.0,
                    colors[i as usize % 3],
                );
            }
            draw_texture_ex(
                &assets.orb,
                e.x - 14.0,
                e.y - 14.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(28.0, 28.0)),
                    rotation: *angle,
                    ..Default::default()
                },
            );
            draw_circle_lines(e.x, e.y, orb_r + 2.0, 1.0, rgba([255, 255, 255], 0.45));
        }
        EntityKind::Clone {
            char_id,
            facing,
            scale,
            hit_cd,
            ..
        } => {
            let texture = assets.slime_for(*char_id);
            let w = 56.0 * scale;
            let h = texture.height() * (w / texture.width());
            let alpha = if *hit_cd > 0.0 { 0.55 } else { 0.8 };
            draw_ellipse(e.x, GROUND_Y - 3.0, 24.0, 5.0, 0.0, rgba([10, 16, 20], 0.2));
            draw_tex(
                texture,
                e.x - w / 2.0,
                e.y + e.h / 2.0 - h,
                w,
                h,
                rgba([190, 255, 210], alpha),
                *facing < 0.0,
                0.0,
            );
        }
    }
}

fn draw_particles(assets: &Assets, m: &Match) {
    for beam in &m.fx.beams {
        let alpha = (beam.life / beam.max_life).clamp(0.0, 1.0);
        let color = rgba(beam.color, alpha * 0.75);
        if beam.dashed {
            let count = 7;
            for i in 0..count {
                if i % 2 == 1 {
                    continue;
                }
                let t0 = i as f32 / count as f32;
                let t1 = (i + 1) as f32 / count as f32;
                draw_line(
                    beam.x1 + (beam.x2 - beam.x1) * t0,
                    beam.y1 + (beam.y2 - beam.y1) * t0,
                    beam.x1 + (beam.x2 - beam.x1) * t1,
                    beam.y1 + (beam.y2 - beam.y1) * t1,
                    beam.width,
                    color,
                );
            }
        } else {
            draw_line(beam.x1, beam.y1, beam.x2, beam.y2, beam.width, color);
        }
    }

    for ring in &m.fx.rings {
        let alpha = (ring.life / ring.max_life).clamp(0.0, 1.0);
        draw_circle_lines(
            ring.x,
            ring.y,
            ring.r,
            ring.width * alpha,
            rgba(ring.color, alpha * 0.8),
        );
    }

    for p in &m.fx.particles {
        let alpha = (p.life / p.max_life).clamp(0.0, 1.0);
        let color = rgba(p.color, alpha);
        match p.shape {
            ParticleShape::Rect => {
                draw_rectangle_ex(
                    p.x,
                    p.y,
                    p.size,
                    p.size,
                    DrawRectangleParams {
                        offset: vec2(0.5, 0.5),
                        rotation: p.rot,
                        color,
                    },
                );
            }
            ParticleShape::Circle => {
                draw_circle(p.x, p.y, p.size * 0.5, color);
            }
            ParticleShape::Spark => {
                draw_line(p.x, p.y, p.x - p.vx * 0.02, p.y - p.vy * 0.02, 1.2, color);
            }
        }
    }

    for popup in &m.fx.popups {
        let age = 1.0 - (popup.life / popup.max_life).clamp(0.0, 1.0);
        let alpha = if age < 0.15 {
            age / 0.15
        } else {
            (popup.life / popup.max_life).min(1.0)
        };
        draw_text_px(
            assets,
            &popup.text,
            popup.x,
            popup.y,
            popup.size / 20.0,
            rgba(popup.color, alpha),
            TextAlign::Center,
            true,
        );
    }
}

fn draw_weather(assets: &Assets, m: &Match) {
    let Some(weather) = m.weather else { return };
    if weather.timer <= 0.0 {
        return;
    }
    let alpha = (weather.timer / 0.5).clamp(0.0, 1.0);
    match weather.kind {
        crate::game::characters::WeatherKind::Rain => {
            let drift = (m.time * 0.7).sin() * 8.0;
            let cloud_x = W / 2.0 - 27.0 + drift;
            draw_tex(
                &assets.rain_cloud,
                cloud_x - 80.0,
                68.0,
                54.0,
                28.0,
                rgba([255, 255, 255], alpha),
                false,
                0.0,
            );
            draw_tex(
                &assets.rain_cloud,
                cloud_x + 34.0,
                74.0,
                54.0,
                28.0,
                rgba([255, 255, 255], alpha * 0.8),
                true,
                0.0,
            );
            draw_tex(
                &assets.rain_cloud,
                cloud_x,
                62.0,
                54.0,
                28.0,
                rgba([255, 255, 255], alpha),
                false,
                0.0,
            );
            for i in 0..44u32 {
                let speed = 250.0 + hash(i + 5) * 150.0;
                let x = (hash(i) * (W + 60.0) + m.time * 28.0).rem_euclid(W + 60.0) - 30.0;
                let y = ((hash(i + 33) * (GROUND_Y - 40.0) + m.time * speed) % (GROUND_Y - 40.0))
                    + 70.0;
                draw_line(
                    x,
                    y,
                    x - 2.5,
                    y + 10.0,
                    1.0,
                    rgba([175, 224, 255], 0.5 * alpha),
                );
            }
            draw_rectangle(
                0.0,
                58.0,
                W,
                GROUND_Y - 58.0,
                rgba([84, 132, 190], 0.08 * alpha),
            );
        }
    }
}

fn draw_freeze(assets: &Assets, m: &Match) {
    let Some(freeze) = m.freeze else { return };
    draw_rectangle(0.0, 58.0, W, GROUND_Y - 58.0, rgba([120, 200, 255], 0.10));
    let p = &m.players[freeze.side];
    let remaining = freeze.timer / freeze.total.max(0.01);
    draw_text_px(
        assets,
        "TIME FROZEN",
        W / 2.0,
        84.0,
        0.55,
        rgba([210, 245, 255], 0.75 + (m.time * 5.0).sin() * 0.15),
        TextAlign::Center,
        true,
    );
    draw_rectangle(
        W / 2.0 - 60.0,
        104.0,
        120.0 * remaining,
        4.0,
        rgba([150, 225, 255], 0.8),
    );
    draw_ellipse(
        p.x,
        p.bottom() - 4.0,
        p.w * 0.8,
        8.0,
        0.0,
        rgba([170, 235, 255], 0.24),
    );
}

pub fn draw_world(assets: &Assets, m: &Match) {
    let arena = &ARENAS[m.arena.min(ARENAS.len() - 1)];
    draw_tex(
        assets.background_for(m.arena),
        0.0,
        0.0,
        W,
        H,
        WHITE,
        false,
        0.0,
    );
    draw_ambient(arena, m.time);

    match m.sport {
        Sport::Soccer => draw_soccer_goals(assets, false),
        Sport::Basketball => draw_baskets(assets),
        Sport::Volleyball => draw_net(assets),
    }

    let mut behind: Vec<&Entity> = Vec::new();
    let mut in_front: Vec<&Entity> = Vec::new();
    for e in m.entities.iter().filter(|e| !e.dead) {
        match e.kind {
            EntityKind::Zone { .. } | EntityKind::Geyser { .. } | EntityKind::Solid { .. } => {
                behind.push(e)
            }
            _ => in_front.push(e),
        }
    }
    for e in behind {
        draw_entity(assets, m, e);
    }

    let (first, second) = if m.players[1].y > m.players[0].y {
        (1usize, 0usize)
    } else {
        (0usize, 1usize)
    };
    draw_player(assets, m, first);
    draw_ball(assets, m);
    draw_player(assets, m, second);

    for e in in_front {
        draw_entity(assets, m, e);
    }

    draw_particles(assets, m);
    draw_freeze(assets, m);
    draw_weather(assets, m);

    if m.sport == Sport::Soccer {
        draw_soccer_goals(assets, true);
    }

    let gl = rgba([255, 245, 210], 0.06);
    draw_rectangle(0.0, CEILING_Y - 4.0, W, 4.0, gl);
}

fn draw_mana_bar(assets: &Assets, x: f32, y: f32, width: f32, ratio: f32, side: usize, cost: f32) {
    draw_rectangle(x - 2.0, y - 2.0, width + 4.0, 13.0, rgba([8, 11, 18], 0.9));
    draw_rectangle_lines(
        x - 2.0,
        y - 2.0,
        width + 4.0,
        13.0,
        1.0,
        rgba([60, 70, 88], 0.9),
    );
    let fill = (width * ratio.clamp(0.0, 1.0)).floor();
    let color = side_color(side);
    draw_rectangle(x, y, fill, 9.0, color);
    for i in 1..5 {
        let tx = x + width * i as f32 / 5.0;
        draw_rectangle(tx, y, 1.0, 9.0, rgba([8, 11, 18], 0.5));
    }
    if cost > 0.0 && cost < 1.0 {
        let marker = x + width * cost;
        draw_rectangle(marker, y - 2.0, 2.0, 13.0, rgba([255, 255, 255], 0.6));
    }
    if ratio >= 0.999 {
        let pulse = ((get_time() as f32) * 3.5).sin() * 0.5 + 0.5;
        draw_rectangle(x, y, fill, 1.5, rgba([255, 255, 255], 0.55 + pulse * 0.35));
    }
    if ratio < cost && cost > 0.0 {
        draw_texture_ex(
            &assets.manabar_glow,
            x,
            y - 10.0,
            rgba([255, 80, 80], 0.12),
            DrawTextureParams {
                dest_size: Some(vec2(width, 26.0)),
                ..Default::default()
            },
        );
    }
}

fn side_label(side: usize) -> &'static str {
    if side == 0 {
        "P1"
    } else {
        "P2"
    }
}

pub fn draw_match_ui(assets: &Assets, m: &Match, paused: bool) {
    let band = rgba([9, 12, 19], 0.92);
    draw_rectangle(0.0, 0.0, W, HUD_H, band);
    draw_rectangle(0.0, HUD_H - 2.0, W, 2.0, rgba([60, 72, 92], 0.8));

    let bar_w = 172.0;
    for side in 0..2 {
        let p = &m.players[side];
        let (x, align) = if side == 0 {
            (12.0, TextAlign::Left)
        } else {
            (W - 12.0, TextAlign::Right)
        };
        let name = format!("{}  {}", side_label(side), p.spec().name.to_uppercase());
        draw_text_px(assets, &name, x, 5.0, 0.42, side_color(side), align, false);
        let bar_x = if side == 0 { x } else { x - bar_w };
        let ratio = p.mana / MANA_MAX;
        let cost = (p.spec().ability.cost / MANA_MAX).clamp(0.0, 1.0);
        draw_mana_bar(assets, bar_x, 22.0, bar_w, ratio, side, cost);
        let ability = p.spec().ability.name.to_uppercase();
        let label = if p.cooldown > 0.0 {
            format!("{} {:.1}s", ability, p.cooldown)
        } else if p.mana + 0.001 < p.spec().ability.cost {
            format!("{} NEEDS {:.0}", ability, p.spec().ability.cost)
        } else {
            ability
        };
        let color = if p.cooldown > 0.0 || p.mana + 0.001 < p.spec().ability.cost {
            rgba([160, 168, 182], 0.9)
        } else {
            rgba([224, 232, 244], 0.9)
        };
        draw_text_px(assets, &label, x, 37.0, 0.34, color, align, false);
    }

    let sport = m.sport.label().to_uppercase();
    draw_text_px(
        assets,
        &sport,
        W / 2.0,
        4.0,
        0.34,
        rgba([170, 184, 204], 0.95),
        TextAlign::Center,
        false,
    );
    draw_text_px(
        assets,
        &format!("{}  :  {}", m.score[0], m.score[1]),
        W / 2.0,
        15.0,
        0.9,
        rgba([255, 250, 236], 1.0),
        TextAlign::Center,
        true,
    );
    let target = format!("FIRST TO {}", m.play_to);
    draw_text_px(
        assets,
        &target,
        W / 2.0,
        47.0,
        0.3,
        rgba([140, 152, 170], 0.9),
        TextAlign::Center,
        false,
    );

    if m.phase == Phase::Countdown {
        let total = m.countdown_total.max(0.01);
        let t = (m.phase_timer / total).clamp(0.0, 1.0);
        let number = m.countdown_number.max(1);
        let pop = 1.0 + (t * total).sin().abs() * 0.08;
        draw_rectangle(0.0, HUD_H, W, GROUND_Y - HUD_H, rgba([5, 8, 14], 0.18));
        draw_text_px(
            assets,
            &number.to_string(),
            W / 2.0,
            142.0,
            3.4 * pop,
            rgba([255, 250, 225], 0.95),
            TextAlign::Center,
            true,
        );
        draw_text_px(
            assets,
            "GET READY",
            W / 2.0,
            258.0,
            0.6,
            rgba([230, 238, 250], 0.85),
            TextAlign::Center,
            true,
        );
    }

    if m.phase == Phase::Playing && m.go_flash > 0.0 {
        let alpha = (m.go_flash / 0.6).clamp(0.0, 1.0);
        draw_text_px(
            assets,
            "GO!",
            W / 2.0,
            138.0,
            3.0 + (1.0 - alpha) * 0.8,
            rgba([255, 244, 160], alpha),
            TextAlign::Center,
            true,
        );
    }

    if m.phase == Phase::Point {
        draw_rectangle(0.0, HUD_H, W, H - HUD_H, rgba([5, 7, 12], 0.55));
        if let Some(info) = m.last_score {
            let word = m.sport.point_word();
            let color = side_color(info.side);
            draw_text_px(
                assets,
                word,
                W / 2.0,
                138.0,
                2.2,
                color,
                TextAlign::Center,
                true,
            );
            let detail = if info.own_goal {
                format!("{} OWN GOAL", side_label(info.side))
            } else if let Some(scorer) = info.scorer {
                format!("{} SCORES", side_label(scorer))
            } else {
                format!("{} SCORES", side_label(info.side))
            };
            draw_text_px(
                assets,
                &detail,
                W / 2.0,
                202.0,
                0.55,
                rgba([235, 242, 250], 0.95),
                TextAlign::Center,
                true,
            );
            draw_text_px(
                assets,
                &format!("{}  -  {}", info.score[0], info.score[1]),
                W / 2.0,
                228.0,
                1.1,
                rgba([255, 255, 255], 1.0),
                TextAlign::Center,
                true,
            );
            if m.pending_end {
                draw_text_px(
                    assets,
                    "FINAL POINT",
                    W / 2.0,
                    278.0,
                    0.5,
                    rgba([255, 220, 140], 0.95),
                    TextAlign::Center,
                    true,
                );
            } else {
                draw_text_px(
                    assets,
                    "NEXT RALLY SOON",
                    W / 2.0,
                    278.0,
                    0.42,
                    rgba([170, 184, 204], 0.9),
                    TextAlign::Center,
                    true,
                );
            }
        }
    }

    if paused {
        draw_pause_overlay(assets);
    }
}

pub fn draw_pause_overlay(assets: &Assets) {
    draw_rectangle(0.0, 0.0, W, H, rgba([4, 6, 11], 0.68));
    panel(
        W / 2.0 - 150.0,
        118.0,
        300.0,
        168.0,
        rgba([15, 20, 31], 0.97),
        rgba([76, 92, 116], 1.0),
    );
    draw_text_px(
        assets,
        "PAUSED",
        W / 2.0,
        138.0,
        1.4,
        rgba([255, 244, 210], 1.0),
        TextAlign::Center,
        true,
    );
    draw_text_px(
        assets,
        "ESC / START  RESUME",
        W / 2.0,
        198.0,
        0.48,
        rgba([215, 226, 240], 0.95),
        TextAlign::Center,
        false,
    );
    draw_text_px(
        assets,
        "Q  QUIT TO MENU",
        W / 2.0,
        222.0,
        0.48,
        rgba([215, 226, 240], 0.95),
        TextAlign::Center,
        false,
    );
    draw_text_px(
        assets,
        "M  MUTE",
        W / 2.0,
        246.0,
        0.48,
        rgba([215, 226, 240], 0.95),
        TextAlign::Center,
        false,
    );
}

pub fn draw_menu_background(assets: &Assets, arena_index: usize, time: f32) {
    let arena = &ARENAS[arena_index.min(ARENAS.len() - 1)];
    draw_tex(
        assets.background_for(arena_index),
        0.0,
        0.0,
        W,
        H,
        rgba([255, 255, 255], 1.0),
        false,
        0.0,
    );
    draw_rectangle(0.0, 0.0, W, H, rgba([6, 9, 16], 0.62));
    draw_ambient(arena, time);
    draw_rectangle(0.0, H - 48.0, W, 48.0, rgba([5, 8, 14], 0.72));
    draw_rectangle(0.0, H - 48.0, W, 2.0, rgba(arena.accent, 0.7));
    let accent = rgba(arena.accent, 0.12 + (time * 1.4).sin() * 0.04);
    draw_circle(W - 78.0, 66.0, 46.0, accent);
    draw_circle(82.0, 326.0, 28.0, rgba(arena.accent, 0.08));
}

pub fn draw_result_backdrop(assets: &Assets, arena_index: usize, winner: usize) {
    draw_menu_background(assets, arena_index, get_time() as f32);
    draw_rectangle(0.0, 0.0, W, H, rgba([4, 7, 13], 0.42));
    let color = side_color(winner);
    draw_rectangle(0.0, 148.0, W, 3.0, color);
    draw_rectangle(0.0, 251.0, W, 3.0, color);
}
