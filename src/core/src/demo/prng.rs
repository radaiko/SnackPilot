//! Deterministic PRNG for demo data (03-features/demo-mode §5.1) — a linear congruential
//! generator that MUST match JavaScript number semantics: the multiply is done in f64 and
//! only the sum is truncated by the 31-bit mask (06-testing §9.3). Values are in [0, 1).
const MAX: f64 = 0x7fff_ffff as f64; // 2147483647

pub struct Lcg {
    s: i64,
}

impl Lcg {
    pub fn new(seed: i64) -> Self {
        Self { s: seed }
    }

    /// Next value in [0, 1). Replicates `s = (s*1103515245 + 12345) & 0x7fffffff; s / 0x7fffffff`
    /// with JS f64 arithmetic: compute in f64 (lossy for large intermediates by design), cast to
    /// i64, then mask the low 31 bits — the same low bits JS's `& 0x7fffffff` keeps.
    pub fn next_f64(&mut self) -> f64 {
        let x = self.s as f64 * 1103515245.0 + 12345.0;
        self.s = (x as i64) & 0x7fff_ffff;
        self.s as f64 / MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_answer_no_precision_loss() {
        // seed=1: 1*1103515245 + 12345 = 1103527590 (< 2^53, exact in f64); < 0x7fffffff so the
        // mask is a no-op → 1103527590 / 2147483647.
        let mut g = Lcg::new(1);
        let expected = 1103527590.0 / 2147483647.0;
        assert!((g.next_f64() - expected).abs() < 1e-12);
    }

    #[test]
    fn deterministic_same_seed_same_sequence() {
        let mut a = Lcg::new(20260210);
        let mut b = Lcg::new(20260210);
        for _ in 0..50 {
            assert_eq!(a.next_f64().to_bits(), b.next_f64().to_bits());
        }
    }

    #[test]
    fn values_in_unit_interval_and_seeds_differ() {
        let mut g = Lcg::new(20260210);
        let mut h = Lcg::new(20260211);
        let mut any_diff = false;
        for _ in 0..1000 {
            let v = g.next_f64();
            assert!((0.0..1.0).contains(&v));
            if (v - h.next_f64()).abs() > f64::EPSILON {
                any_diff = true;
            }
        }
        assert!(any_diff, "different seeds should diverge");
    }

    #[test]
    fn large_seed_exercises_lossy_path_and_stays_in_range() {
        // a seed large enough that s*1103515245 exceeds 2^53 (f64 precision loss is part of
        // the algorithm) — the result must still be a well-formed value in [0, 1).
        let mut g = Lcg::new(2_000_000_000);
        for _ in 0..100 {
            let v = g.next_f64();
            assert!((0.0..1.0).contains(&v));
        }
    }
}
