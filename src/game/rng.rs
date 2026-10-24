use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rng {
    state: u32,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut s = seed as u32;
        s ^= (seed >> 32) as u32;
        if s == 0 {
            s = 0x9e37_79b9;
        }
        Self { state: s }
    }

    pub fn from_parts(a: u64, b: u64) -> Self {
        Self::new(a.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ b.rotate_left(17))
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        t ^ (t >> 14)
    }

    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.next_f32() < p
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next_u32() as usize) % items.len()]
    }
}
