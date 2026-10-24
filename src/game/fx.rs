use super::config::GROUND_Y;
use super::math::clamp;
use super::rng::Rng;

const MAX_PARTICLES: usize = 620;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleShape {
    Rect,
    Circle,
    Spark,
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub color: [u8; 3],
    pub gravity: f32,
    pub shape: ParticleShape,
    pub fade: bool,
    pub spin: f32,
    pub rot: f32,
}

#[derive(Clone, Debug)]
pub struct Ring {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub max_r: f32,
    pub life: f32,
    pub max_life: f32,
    pub color: [u8; 3],
    pub width: f32,
}

#[derive(Clone, Debug)]
pub struct Beam {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub life: f32,
    pub max_life: f32,
    pub color: [u8; 3],
    pub width: f32,
    pub dashed: bool,
}

#[derive(Clone, Debug)]
pub struct Popup {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub color: [u8; 3],
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Fx {
    pub particles: Vec<Particle>,
    pub rings: Vec<Ring>,
    pub beams: Vec<Beam>,
    pub popups: Vec<Popup>,
}

impl Fx {
    #[allow(clippy::too_many_arguments)]
    pub fn burst(
        &mut self,
        rng: &mut Rng,
        x: f32,
        y: f32,
        colors: &[[u8; 3]],
        count: usize,
        speed: f32,
        life: f32,
        gravity: f32,
        size: f32,
        angle: f32,
        spread: f32,
        shape: ParticleShape,
    ) {
        for _ in 0..count {
            let a = angle + rng.range(-spread / 2.0, spread / 2.0);
            let v = rng.range(speed * 0.25, speed);
            self.particle(Particle {
                x: x + rng.range(-3.0, 3.0),
                y: y + rng.range(-3.0, 3.0),
                vx: a.cos() * v,
                vy: a.sin() * v,
                life: rng.range(life * 0.55, life),
                max_life: life,
                size: rng.range(size * 0.6, size * 1.5),
                color: colors[(rng.next_u32() as usize) % colors.len()],
                gravity,
                shape,
                fade: true,
                spin: rng.range(-12.0, 12.0),
                rot: 0.0,
            });
        }
    }

    pub fn particle(&mut self, particle: Particle) {
        if self.particles.len() >= MAX_PARTICLES {
            self.particles.remove(0);
        }
        self.particles.push(particle);
    }

    pub fn confetti(&mut self, rng: &mut Rng, x: f32, y: f32, dir: f32) {
        let colors = [
            [255, 93, 143],
            [255, 213, 74],
            [142, 230, 79],
            [75, 229, 255],
            [176, 108, 232],
            [255, 255, 255],
        ];
        for _ in 0..70 {
            let a = -std::f32::consts::FRAC_PI_2 + rng.range(-1.5, 1.5);
            let v = rng.range(100.0, 320.0);
            self.particle(Particle {
                x,
                y,
                vx: a.cos() * v + dir * rng.range(20.0, 90.0),
                vy: a.sin() * v,
                life: rng.range(0.9, 1.8),
                max_life: 1.8,
                size: rng.range(2.0, 4.0),
                color: colors[(rng.next_u32() as usize) % colors.len()],
                gravity: 380.0,
                shape: ParticleShape::Rect,
                fade: true,
                spin: rng.range(-12.0, 12.0),
                rot: 0.0,
            });
        }
    }

    pub fn ring(&mut self, x: f32, y: f32, max_r: f32, color: [u8; 3], life: f32, width: f32) {
        self.rings.push(Ring {
            x,
            y,
            r: 4.0,
            max_r,
            life,
            max_life: life,
            color,
            width,
        });
    }

    pub fn popup(
        &mut self,
        text: impl Into<String>,
        x: f32,
        y: f32,
        color: [u8; 3],
        life: f32,
        size: f32,
    ) {
        if self.popups.len() > 24 {
            self.popups.remove(0);
        }
        self.popups.push(Popup {
            text: text.into(),
            x,
            y,
            color,
            life,
            max_life: life,
            size,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn beam(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: [u8; 3],
        life: f32,
        width: f32,
        dashed: bool,
    ) {
        self.beams.push(Beam {
            x1,
            y1,
            x2,
            y2,
            life,
            max_life: life,
            color,
            width,
            dashed,
        });
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.life -= dt;
            p.vy += p.gravity * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.rot += p.spin * dt;
            if p.y > GROUND_Y - 1.0 && p.vy > 0.0 {
                p.y = GROUND_Y - 1.0;
                p.vy *= -0.35;
                p.vx *= 0.7;
            }
        }
        self.particles.retain(|p| p.life > 0.0);

        for r in &mut self.rings {
            r.life -= dt;
            let t = clamp(1.0 - r.life / r.max_life, 0.0, 1.0);
            r.r = 4.0 + (r.max_r - 4.0) * t;
        }
        self.rings.retain(|r| r.life > 0.0);

        for b in &mut self.beams {
            b.life -= dt;
        }
        self.beams.retain(|b| b.life > 0.0);

        for p in &mut self.popups {
            p.life -= dt;
            p.y -= 24.0 * dt;
        }
        self.popups.retain(|p| p.life > 0.0);
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.rings.clear();
        self.beams.clear();
        self.popups.clear();
    }
}
