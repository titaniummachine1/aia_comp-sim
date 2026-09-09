//! Tennis RNG streams — mirrored from `tennis-sim/native/core.hpp` (SplitMix64
//! VM streams) and `recovered_random.hpp` (Unity xorshift128, bit-exact).

/// SplitMix64 — the experiment/VM stream (`core.hpp` RNG).
/// Per-player streams are seeded `seed ^ 0x83a2f347` (home) and
/// `seed ^ 0x157cc26d` (away).
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform float in [0, 1): top 24 bits scaled (`unit()`).
    pub fn unit(&mut self) -> f32 {
        ((self.next() >> 40) as f32) * 2f32.powi(-24)
    }

    /// `rng.next() % n` as used by the model (first server, random shot).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// Unity `ReferenceRandom` xorshift128 — verified bit-for-bit on 1,024 draws
/// against the game (`specs/reference-validation.md`). Used by fatigue, trick
/// variants, and random aim rerolls.
#[derive(Debug, Clone)]
pub struct UnityRandom {
    s: [u32; 4],
}

impl UnityRandom {
    /// `s[0] = seed; s[i] = 1812433253*s[i-1] + 1`.
    pub fn new(seed: u32) -> Self {
        let mut s = [0u32; 4];
        s[0] = seed;
        for i in 1..4 {
            s[i] = 1812433253u32
                .wrapping_mul(s[i - 1])
                .wrapping_add(1);
        }
        Self { s }
    }

    pub fn next_u32(&mut self) -> u32 {
        // xorshift128 (Unity Mathf-level generator).
        let mut t = self.s[3];
        let s = self.s[0];
        self.s[3] = self.s[2];
        self.s[2] = self.s[1];
        self.s[1] = s;
        t ^= t << 11;
        t ^= t >> 8;
        self.s[0] = t ^ s ^ (s >> 18);
        self.s[0]
    }

    /// `range(min,max) = (1-t)*max + t*min` with `t = (next()&0x7fffff)*2^-23`.
    /// Consumes a draw even when the bounds are equal (game-faithful).
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        let t = ((self.next_u32() & 0x7f_ffff) as f32) * 2f32.powi(-23);
        (1.0 - t) * max + t * min
    }

    /// Unity `RandomRangeInt(min, max)` inclusive — `min + floor(t*(max-min+1))`.
    pub fn range_int(&mut self, min: i32, max: i32) -> i32 {
        let t = ((self.next_u32() & 0x7f_ffff) as f32) * 2f32.powi(-23);
        min + (t * ((max - min + 1) as f32)).floor() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix_is_deterministic() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..16 {
            assert_eq!(a.next(), b.next());
        }
        let mut c = SplitMix64::new(43);
        assert_ne!(a.next(), c.next());
    }

    #[test]
    fn unity_random_initial_state_matches_the_game_capture() {
        // Natural v0.14 trace: Random.InitState(20260907) →
        // [20260907, 698670072, 1267368153, 4270323358].
        let r = UnityRandom::new(20260907);
        assert_eq!(r.s, [20260907, 698670072, 1267368153, 4270323358]);
    }

    #[test]
    fn unity_range_stays_in_bounds_and_consumes_one_draw() {
        let mut r = UnityRandom::new(7);
        for _ in 0..1000 {
            let v = r.range(-1.0, 1.0);
            assert!((-1.0..=1.0).contains(&v), "{v}");
        }
    }
}
