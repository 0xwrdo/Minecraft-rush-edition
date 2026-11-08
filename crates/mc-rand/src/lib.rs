//! Clean-room implementation of Java's Linear Congruential Generator (LCG).
//!
//! Based strictly on documented behavior from the Java SE 6 / 7 API specification
//! for `java.util.Random`, which Minecraft 1.0.0 uses for world generation,
//! chunk population, cave carvers, and entity RNG.
//!
//! Has NO graphics dependencies and runs entirely headless.

/// 48-bit Linear Congruential Generator matching `java.util.Random`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JavaRandom {
    seed: u64,
    next_next_gaussian: f64,
    have_next_next_gaussian: bool,
}

const MULTIPLIER: u64 = 0x5DEECE66D;
const ADDEND: u64 = 0xB;
const MASK: u64 = (1u64 << 48) - 1;

impl JavaRandom {
    /// Constructs a new `JavaRandom` using a 64-bit integer seed.
    /// Java spec: scrambled initial seed is `(seed ^ 0x5DEECE66DL) & ((1L << 48) - 1)`
    #[inline]
    pub fn new(seed: i64) -> Self {
        let mut rng = Self {
            seed: 0,
            next_next_gaussian: 0.0,
            have_next_next_gaussian: false,
        };
        rng.set_seed(seed);
        rng
    }

    /// Sets the seed of this random number generator.
    /// Java spec: `seed = (seed ^ MULTIPLIER) & MASK`, clears Gaussian cache.
    #[inline]
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = ((seed as u64) ^ MULTIPLIER) & MASK;
        self.have_next_next_gaussian = false;
        self.next_next_gaussian = 0.0;
    }

    /// Generates the next pseudorandom number of `bits` (1 to 32).
    /// Java spec:
    /// `seed = (seed * 0x5DEECE66DL + 0xBL) & ((1L << 48) - 1)`
    /// Returns: `(int)(seed >>> (48 - bits))`
    #[inline]
    pub fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        (self.seed >> (48 - bits)) as i32
    }

    /// Returns the next pseudorandom, uniformly distributed `i32` value.
    /// Java spec: `next(32)`
    #[inline]
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// Returns a pseudorandom, uniformly distributed `i32` value between 0 (inclusive)
    /// and the specified `bound` (exclusive).
    ///
    /// Implements both the power-of-two fast path and the rejection loop from Java spec:
    /// - If `bound` is power of two: `(int)((bound * (long)next(31)) >> 31)`
    /// - Otherwise: sample `next(31)` and reject values if `bits - val + (bound - 1) < 0`
    #[inline]
    pub fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");

        // Java spec power-of-two optimization: (bound & -bound) == bound
        #[allow(clippy::manual_isolate_lowest_one)]
        if (bound & -bound) == bound {
            return (((bound as i64) * (self.next(31) as i64)) >> 31) as i32;
        }

        let mut bits;
        let mut val;
        loop {
            bits = self.next(31);
            val = bits % bound;
            if bits - val + (bound - 1) >= 0 {
                break;
            }
        }
        val
    }

    /// Returns the next pseudorandom, uniformly distributed `i64` value.
    /// Java spec: `((long)next(32) << 32) + next(32)`
    #[inline]
    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }

    /// Returns the next pseudorandom, uniformly distributed `bool` value.
    /// Java spec: `next(1) != 0`
    #[inline]
    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }

    /// Returns the next pseudorandom, uniformly distributed `f32` value in `[0.0, 1.0)`.
    /// Java spec: `next(24) / ((float)(1 << 24))`
    #[inline]
    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / ((1 << 24) as f32)
    }

    /// Returns the next pseudorandom, uniformly distributed `f64` value in `[0.0, 1.0)`.
    /// Java spec: `(((long)next(26) << 27) + next(27)) / (double)(1L << 53)`
    #[inline]
    pub fn next_double(&mut self) -> f64 {
        let high = (self.next(26) as i64) << 27;
        let low = self.next(27) as i64;
        high.wrapping_add(low) as f64 / ((1u64 << 53) as f64)
    }

    /// Returns the next pseudorandom, Gaussian distributed `f64` value
    /// with mean `0.0` and standard deviation `1.0`.
    ///
    /// Uses the polar form of the Box-Muller transformation and caches the second value.
    ///
    /// // TODO(verify): Java's java.util.Random uses StrictMath.log and StrictMath.sqrt.
    /// // Standard IEEE 754 f64::ln and f64::sqrt match x86_64 StrictMath implementations,
    /// // but verify if non-x86 hardware or different libm implementations introduce 1-ulp drifts.
    pub fn next_gaussian(&mut self) -> f64 {
        if self.have_next_next_gaussian {
            self.have_next_next_gaussian = false;
            self.next_next_gaussian
        } else {
            let mut v1: f64;
            let mut v2: f64;
            let mut s: f64;
            loop {
                v1 = 2.0 * self.next_double() - 1.0;
                v2 = 2.0 * self.next_double() - 1.0;
                s = v1 * v1 + v2 * v2;
                if s < 1.0 && s != 0.0 {
                    break;
                }
            }
            // TODO(verify): StrictMath.sqrt(-2 * StrictMath.log(s) / s)
            let multiplier = (-2.0 * s.ln() / s).sqrt();
            self.next_next_gaussian = v2 * multiplier;
            self.have_next_next_gaussian = true;
            v1 * multiplier
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_0_first_two_next_int() {
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next_int(), -1155484576);
        assert_eq!(r.next_int(), -723955400);
    }

    #[test]
    fn test_seed_42_first_two_next_int() {
        let mut r = JavaRandom::new(42);
        assert_eq!(r.next_int(), -1170105035);
        assert_eq!(r.next_int(), 234785527);
    }

    #[test]
    fn test_power_of_two_and_rejection_paths() {
        let mut r = JavaRandom::new(1337);
        // Power of two branch
        let pow2 = r.next_int_bounded(64);
        assert!((0..64).contains(&pow2));

        // Non-power of two (rejection loop branch)
        let non_pow2 = r.next_int_bounded(99);
        assert!((0..99).contains(&non_pow2));
    }

    #[test]
    fn test_primitives_range() {
        let mut r = JavaRandom::new(9999);
        for _ in 0..100 {
            let f = r.next_float();
            assert!((0.0..1.0).contains(&f));

            let d = r.next_double();
            assert!((0.0..1.0).contains(&d));

            let _b = r.next_boolean();
            let _l = r.next_long();
        }
    }

    #[test]
    fn test_gaussian_cache_toggle() {
        let mut r = JavaRandom::new(777);
        assert!(!r.have_next_next_gaussian);

        let _g1 = r.next_gaussian();
        assert!(r.have_next_next_gaussian);

        let _g2 = r.next_gaussian();
        assert!(!r.have_next_next_gaussian);
    }
}
