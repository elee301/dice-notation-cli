use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// Bumped on every Rng::new() so two rolls started in the same process within
// the same nanosecond still get different seeds.
static CALL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A small, self-seeded PRNG. Not cryptographically secure, and not meant to
/// be - it only needs to spread rolls evenly and avoid repeating the same
/// sequence across separate invocations of the tool.
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let pid = std::process::id() as u64;
        let call = CALL_COUNTER.fetch_add(1, Ordering::Relaxed);

        let seed = nanos
            ^ pid.rotate_left(32)
            ^ call.wrapping_mul(0x9E3779B97F4A7C15);

        // splitmix64 misbehaves on a zero state, so force it odd.
        Rng { state: seed | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Rolls a single die with the given number of sides, returning a value
    /// in 1..=sides. A zero-sided die always returns 0.
    pub fn roll_die(&mut self, sides: u32) -> u32 {
        if sides == 0 {
            return 0;
        }
        (self.next_u64() % sides as u64) as u32 + 1
    }
}
