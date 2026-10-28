use std::time::{SystemTime, UNIX_EPOCH};

/// splitmix64 — mały, szybki i wystarczająco losowy do animacji.
pub struct Rng(u64);

impl Rng {
    /// Nowe ziarno przy każdej przemianie, żeby żadne dwie nie wyglądały tak samo.
    pub fn from_time() -> Rng {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(2137, |d| d.as_nanos() as u64);
        Rng(nanos ^ ((std::process::id() as u64) << 32))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// [0, 1)
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }

    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            v.swap(i, self.below(i + 1));
        }
    }
}
