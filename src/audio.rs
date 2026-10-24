use macroquad::audio::{self, PlaySoundParams, Sound};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sfx {
    Kick,
    Head,
    Jump,
    Ability,
    Bounce,
    Goal,
    Whistle,
    Ui,
    Deny,
    Explosion,
    Basket,
    Net,
    Post,
    Countdown,
    Go,
    Win,
    Lose,
}

const SFX_COUNT: usize = 17;
const RATE: u32 = 22_050;

pub struct Audio {
    sfx: Vec<Option<Sound>>,
    music: Option<Sound>,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub muted: bool,
}

fn wav_bytes(samples: &[f32], rate: u32) -> Vec<u8> {
    let data_len = samples.len() as u32 * 2;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &sample in samples {
        let v = (sample.clamp(-1.0, 1.0) * 32_767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn square(phase: f32) -> f32 {
    if phase.sin() >= 0.0 {
        1.0
    } else {
        -1.0
    }
}

fn triangle(phase: f32) -> f32 {
    let x = phase / std::f32::consts::TAU;
    4.0 * (x - (x + 0.5).floor()).abs() - 1.0
}

#[allow(clippy::too_many_arguments)]
fn tone(
    buf: &mut [f32],
    start: f32,
    duration: f32,
    f0: f32,
    f1: f32,
    volume: f32,
    wave: fn(f32) -> f32,
    rate: u32,
) {
    let begin = (start * rate as f32) as usize;
    let count = (duration * rate as f32) as usize;
    let mut phase = 0.0f32;
    for i in 0..count {
        let idx = begin + i;
        if idx >= buf.len() {
            break;
        }
        let t = i as f32 / rate as f32;
        let k = (t / duration).clamp(0.0, 1.0);
        let freq = f0 + (f1 - f0) * k;
        phase += std::f32::consts::TAU * freq / rate as f32;
        let attack = (t / 0.008).min(1.0);
        let release = (1.0 - k).powf(1.8);
        buf[idx] += wave(phase) * volume * attack * release;
    }
}

fn noise(buf: &mut [f32], start: f32, duration: f32, volume: f32, lowpass: f32, rate: u32) {
    let begin = (start * rate as f32) as usize;
    let count = (duration * rate as f32) as usize;
    let mut state = 0x1234_5678u32;
    let mut last = 0.0f32;
    for i in 0..count {
        let idx = begin + i;
        if idx >= buf.len() {
            break;
        }
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let raw = (state >> 8) as f32 / 8_388_608.0 - 1.0;
        last = last + lowpass * (raw - last);
        let t = i as f32 / rate as f32;
        let release = (1.0 - (t / duration).min(1.0)).powf(2.0);
        buf[idx] += last * volume * release;
    }
}

fn make_sfx(kind: Sfx) -> Vec<f32> {
    let mut buf = vec![0.0f32; (RATE as f32 * 1.2) as usize];
    match kind {
        Sfx::Kick => {
            tone(&mut buf, 0.0, 0.09, 210.0, 95.0, 0.55, square, RATE);
            noise(&mut buf, 0.0, 0.045, 0.3, 0.4, RATE);
        }
        Sfx::Head => {
            tone(&mut buf, 0.0, 0.11, 330.0, 150.0, 0.5, triangle, RATE);
            noise(&mut buf, 0.0, 0.05, 0.22, 0.5, RATE);
        }
        Sfx::Jump => {
            tone(&mut buf, 0.0, 0.1, 300.0, 620.0, 0.32, square, RATE);
        }
        Sfx::Ability => {
            tone(&mut buf, 0.0, 0.09, 520.0, 760.0, 0.3, square, RATE);
            tone(&mut buf, 0.07, 0.12, 760.0, 1080.0, 0.24, square, RATE);
        }
        Sfx::Bounce => {
            tone(&mut buf, 0.0, 0.07, 150.0, 85.0, 0.34, triangle, RATE);
        }
        Sfx::Goal => {
            let notes = [523.25, 659.25, 783.99, 1046.5, 1318.5];
            for (i, note) in notes.iter().enumerate() {
                tone(
                    &mut buf,
                    i as f32 * 0.1,
                    0.18,
                    *note,
                    *note,
                    0.32,
                    square,
                    RATE,
                );
            }
            tone(&mut buf, 0.5, 0.55, 1046.5, 1046.5, 0.2, triangle, RATE);
            noise(&mut buf, 0.0, 0.5, 0.08, 0.15, RATE);
        }
        Sfx::Whistle => {
            tone(&mut buf, 0.0, 0.55, 2050.0, 2200.0, 0.22, square, RATE);
            tone(&mut buf, 0.0, 0.55, 2080.0, 2240.0, 0.12, triangle, RATE);
        }
        Sfx::Ui => {
            tone(&mut buf, 0.0, 0.045, 880.0, 1100.0, 0.18, square, RATE);
        }
        Sfx::Deny => {
            tone(&mut buf, 0.0, 0.16, 130.0, 90.0, 0.28, square, RATE);
        }
        Sfx::Explosion => {
            noise(&mut buf, 0.0, 0.5, 0.65, 0.12, RATE);
            tone(&mut buf, 0.0, 0.35, 90.0, 40.0, 0.4, triangle, RATE);
        }
        Sfx::Basket => {
            noise(&mut buf, 0.0, 0.14, 0.2, 0.35, RATE);
            tone(&mut buf, 0.05, 0.18, 760.0, 1220.0, 0.28, square, RATE);
        }
        Sfx::Net => {
            noise(&mut buf, 0.0, 0.12, 0.2, 0.5, RATE);
            tone(&mut buf, 0.0, 0.08, 260.0, 180.0, 0.15, triangle, RATE);
        }
        Sfx::Post => {
            tone(&mut buf, 0.0, 0.35, 1180.0, 960.0, 0.26, triangle, RATE);
            tone(&mut buf, 0.0, 0.26, 1760.0, 1500.0, 0.14, triangle, RATE);
            tone(&mut buf, 0.0, 0.16, 2360.0, 2100.0, 0.07, square, RATE);
        }
        Sfx::Countdown => {
            tone(&mut buf, 0.0, 0.12, 440.0, 440.0, 0.24, square, RATE);
        }
        Sfx::Go => {
            tone(&mut buf, 0.0, 0.25, 880.0, 880.0, 0.28, square, RATE);
            tone(&mut buf, 0.0, 0.25, 1320.0, 1320.0, 0.12, triangle, RATE);
        }
        Sfx::Win => {
            let notes = [523.25, 659.25, 783.99, 1046.5, 783.99, 1046.5, 1318.5];
            for (i, note) in notes.iter().enumerate() {
                tone(
                    &mut buf,
                    i as f32 * 0.13,
                    0.22,
                    *note,
                    *note,
                    0.28,
                    square,
                    RATE,
                );
            }
        }
        Sfx::Lose => {
            let notes = [392.0, 349.23, 293.66, 261.63];
            for (i, note) in notes.iter().enumerate() {
                tone(
                    &mut buf,
                    i as f32 * 0.17,
                    0.3,
                    *note,
                    *note,
                    0.24,
                    triangle,
                    RATE,
                );
            }
        }
    }
    buf
}

fn make_music() -> Vec<f32> {
    let bars = 16;
    let step = 0.24f32;
    let duration = bars as f32 * 4.0 * step;
    let mut buf = vec![0.0f32; (duration * RATE as f32) as usize + RATE as usize];

    let bass = [130.81, 130.81, 174.61, 196.0, 130.81, 146.83, 174.61, 196.0];
    let lead = [
        523.25, 659.25, 783.99, 659.25, 587.33, 523.25, 587.33, 659.25, 523.25, 659.25, 783.99,
        1046.5, 987.77, 783.99, 659.25, 587.33,
    ];
    for bar in 0..bars {
        let start = bar as f32 * 4.0 * step;
        let root = bass[bar % bass.len()];
        tone(
            &mut buf,
            start,
            step * 1.8,
            root / 2.0,
            root / 2.0,
            0.16,
            triangle,
            RATE,
        );
        tone(
            &mut buf,
            start + step * 2.0,
            step * 1.8,
            root / 2.0,
            root / 2.0,
            0.12,
            triangle,
            RATE,
        );
        for i in 0..2 {
            let n = lead[(bar * 2 + i) % lead.len()];
            tone(
                &mut buf,
                start + i as f32 * step,
                step * 0.82,
                n,
                n,
                0.1,
                square,
                RATE,
            );
        }
        for i in 0..4 {
            noise(
                &mut buf,
                start + i as f32 * step,
                0.035,
                0.03 + (i % 2) as f32 * 0.02,
                0.9,
                RATE,
            );
        }
    }
    for sample in &mut buf {
        *sample = (*sample * 0.9).tanh();
    }
    buf
}

impl Audio {
    pub async fn load(settings: &crate::save::GameSettings) -> Self {
        let mut sfx = Vec::with_capacity(SFX_COUNT);
        let kinds = [
            Sfx::Kick,
            Sfx::Head,
            Sfx::Jump,
            Sfx::Ability,
            Sfx::Bounce,
            Sfx::Goal,
            Sfx::Whistle,
            Sfx::Ui,
            Sfx::Deny,
            Sfx::Explosion,
            Sfx::Basket,
            Sfx::Net,
            Sfx::Post,
            Sfx::Countdown,
            Sfx::Go,
            Sfx::Win,
            Sfx::Lose,
        ];
        for kind in kinds {
            let bytes = wav_bytes(&make_sfx(kind), RATE);
            sfx.push(audio::load_sound_from_bytes(&bytes).await.ok());
        }
        let music_bytes = wav_bytes(&make_music(), RATE);
        let music = audio::load_sound_from_bytes(&music_bytes).await.ok();
        if let Some(music) = &music {
            audio::play_sound(
                music,
                PlaySoundParams {
                    looped: true,
                    volume: settings.music * 0.45,
                },
            );
        }
        Self {
            sfx,
            music,
            music_volume: settings.music,
            sfx_volume: settings.sfx,
            muted: false,
        }
    }

    pub fn play(&self, kind: Sfx) {
        if self.muted {
            return;
        }
        let index = kind as usize;
        if let Some(Some(sound)) = self.sfx.get(index) {
            audio::play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume: self.sfx_volume * 0.7,
                },
            );
        }
    }

    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume.clamp(0.0, 1.0);
        if let Some(music) = &self.music {
            let volume = if self.muted {
                0.0
            } else {
                self.music_volume * 0.45
            };
            audio::set_sound_volume(music, volume);
        }
    }

    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        if let Some(music) = &self.music {
            let volume = if self.muted {
                0.0
            } else {
                self.music_volume * 0.45
            };
            audio::set_sound_volume(music, volume);
        }
    }

    pub fn toggle_muted(&mut self) -> bool {
        self.set_muted(!self.muted);
        self.muted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: [Sfx; SFX_COUNT] = [
        Sfx::Kick,
        Sfx::Head,
        Sfx::Jump,
        Sfx::Ability,
        Sfx::Bounce,
        Sfx::Goal,
        Sfx::Whistle,
        Sfx::Ui,
        Sfx::Deny,
        Sfx::Explosion,
        Sfx::Basket,
        Sfx::Net,
        Sfx::Post,
        Sfx::Countdown,
        Sfx::Go,
        Sfx::Win,
        Sfx::Lose,
    ];

    #[test]
    fn every_sound_has_audio() {
        for kind in KINDS {
            let samples = make_sfx(kind);
            assert!(!samples.is_empty(), "{kind:?} is empty");
            assert!(samples.iter().all(|s| s.is_finite()), "{kind:?} has NaN");
            let peak = samples.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
            assert!(peak > 0.01, "{kind:?} is silent (peak {peak})");
            assert!(peak <= 1.0, "{kind:?} clips at {peak}");
        }
    }

    #[test]
    fn wav_header_matches_the_samples() {
        let samples = make_sfx(Sfx::Kick);
        let bytes = wav_bytes(&samples, RATE);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(u16::from_le_bytes([bytes[20], bytes[21]]), 1);
        assert_eq!(
            u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            RATE
        );
        assert_eq!(bytes.len(), 44 + samples.len() * 2);
        assert_eq!(
            u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]) as usize,
            samples.len() * 2
        );
    }

    #[test]
    fn music_loop_is_long_and_finite() {
        let music = make_music();
        assert!(music.len() > RATE as usize * 12);
        assert!(music.iter().all(|s| s.is_finite()));
        let peak = music.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
        assert!(peak > 0.05 && peak <= 1.0);
    }
}
