//! Clean-room implementation of Java's Linear Congruential Generator (LCG).
//!
//! Based strictly on documented behavior from the Java SE 6 / 7 API specification
//! for `java.util.Random`, which Minecraft 1.0.0 uses for terrain generation,
//! chunk population, carvers, and entity behavior.
//!
//! Has NO graphics dependencies and runs entirely headless.

/// Clean-room implementation of `java.util.Random`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JavaRandom {
    seed: u64,
    next_next_gaussian: u64, // Stored as u64 bit representation of f64 for Eq/PartialEq
    have_next_next_gaussian: bool,
}

const MULTIPLIER: u64 = 0x5DEECE66D;
const ADDEND: u64 = 0xB;
const MASK: u64 = (1u64 << 48) - 1;

impl JavaRandom {
    /// Constructs a new `JavaRandom` with initial seed.
    /// Java spec: `(seed ^ multiplier) & mask`
    #[inline]
    pub fn new(seed: i64) -> Self {
        let mut rng = Self {
            seed: 0,
            next_next_gaussian: 0,
            have_next_next_gaussian: false,
        };
        rng.set_seed(seed);
        rng
    }

    /// Sets the seed of this random number generator using a single long seed.
    #[inline]
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = ((seed as u64) ^ MULTIPLIER) & MASK;
        self.have_next_next_gaussian = false;
    }

    /// Generates the next pseudorandom number of `bits` (1 to 32).
    /// Java spec: `seed = (seed * 0x5DEECE66DL + 0xBL) & ((1L << 48) - 1)`
    /// Returns: `(int)(seed >>> (48 - bits))`
    #[inline]
    pub fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        (self.seed >> (48 - bits)) as i32
    }

    /// Returns the next pseudorandom, uniformly distributed int value.
    #[inline]
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// Returns a pseudorandom, uniformly distributed int value between 0 (inclusive)
    /// and the specified value (exclusive).
    #[inline]
    pub fn next_int_bounded(&mut self, n: i32) -> i32 {
        assert!(n > 0, "n must be positive");

        // Java spec power-of-two optimization: (n & -n) == n
        #[allow(clippy::manual_isolate_lowest_one)]
        if (n & -n) == n {
            return (((n as i64) * (self.next(31) as i64)) >> 31) as i32;
        }

        let mut bits;
        let mut val;
        loop {
            bits = self.next(31);
            val = bits % n;
            if bits - val + (n - 1) >= 0 {
                break;
            }
        }
        val
    }

    /// Returns the next pseudorandom, uniformly distributed long value.
    /// Java spec: `((long)next(32) << 32) + next(32)`
    #[inline]
    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }

    /// Returns the next pseudorandom, uniformly distributed boolean value.
    #[inline]
    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }

    /// Returns the next pseudorandom, uniformly distributed float value between 0.0 and 1.0.
    #[inline]
    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / ((1 << 24) as f32)
    }

    /// Returns the next pseudorandom, uniformly distributed double value between 0.0 and 1.0.
    /// Java spec: `(((long)next(26) << 27) + next(27)) / (double)(1L << 53)`
    #[inline]
    pub fn next_double(&mut self) -> f64 {
        let high = (self.next(26) as i64) << 27;
        let low = self.next(27) as i64;
        high.wrapping_add(low) as f64 / ((1u64 << 53) as f64)
    }

    /// Returns the next pseudorandom, Gaussian ("normally") distributed double value
    /// with mean 0.0 and standard deviation 1.0 from this random number generator's sequence.
    /// Uses the polar form of the Box-Muller transformation documented in the Java SE API.
    pub fn next_gaussian(&mut self) -> f64 {
        if self.have_next_next_gaussian {
            self.have_next_next_gaussian = false;
            f64::from_bits(self.next_next_gaussian)
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
            let multiplier = (-2.0 * s.ln() / s).sqrt();
            self.next_next_gaussian = (v2 * multiplier).to_bits();
            self.have_next_next_gaussian = true;
            v1 * multiplier
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_0_ground_truth() {
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next_int_bounded(100), 60);
        assert!((r.next_double() - 0.8314409887870612).abs() < 1e-15);
        assert_eq!(r.next_long(), -7261648964369397258);
        assert!((r.next_gaussian() - (-0.35199821740360626)).abs() < 1e-15);

        assert_eq!(r.next_int_bounded(100), 54);
        assert!((r.next_double() - 0.3332183994766498).abs() < 1e-15);
        assert_eq!(r.next_long(), 7105486291024734541);
        assert!((r.next_gaussian() - (-0.7060140986343248)).abs() < 1e-15);
    }

    #[test]
    fn test_seed_42_ground_truth() {
        let mut r = JavaRandom::new(42);
        assert_eq!(r.next_int_bounded(100), 30);
        assert!((r.next_double() - 0.05466526274716077).abs() < 1e-15);
        assert_eq!(r.next_long(), 884324181205335268);
        assert!((r.next_gaussian() - 0.27686040089698144).abs() < 1e-15);

        assert_eq!(r.next_int_bounded(100), 93);
        assert!((r.next_double() - 0.9033722646721782).abs() < 1e-15);
        assert_eq!(r.next_long(), 6802844026563419272);
        assert!((r.next_gaussian() - 0.13008432156952723).abs() < 1e-15);
    }

    #[test]
    fn test_seed_12345_and_negative_seed() {
        let mut r = JavaRandom::new(12345);
        assert_eq!(r.next_int_bounded(100), 51);
        assert!((r.next_double() - 0.5132095356731635).abs() < 1e-15);
        assert_eq!(r.next_long(), -1528963862231680626);
        assert!((r.next_gaussian() - 0.2367921177555527).abs() < 1e-15);

        let mut r_neg = JavaRandom::new(-987654321);
        assert_eq!(r_neg.next_int_bounded(100), 62);
        assert!((r_neg.next_double() - 0.6162466198738454).abs() < 1e-15);
        assert_eq!(r_neg.next_long(), -4449053698085500532);
        assert!((r_neg.next_gaussian() - (-0.14330858453653078)).abs() < 1e-15);
    }
}
