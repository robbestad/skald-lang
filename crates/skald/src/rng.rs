const MULTIPLIER: u64 = 6364136223846793005;
const INCREMENT: u64 = 1442695040888963407;

/// PCG32. Same algorithm on native and WASM so seeds stay portable inside Skald.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn from_u64(seed: u64) -> Self {
        let mut rng = Self { state: 0 };
        rng.state = seed.wrapping_add(INCREMENT);
        let _ = rng.next_u32();
        rng
    }

    pub fn from_seed(seed: Option<Seed>) -> Self {
        match seed {
            Some(Seed::Int(n)) => Self::from_u64(n),
            Some(Seed::Text(s)) => Self::from_u64(hash_str(&s)),
            None => Self::from_u64(unseeded()),
        }
    }

    fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform in `[0, 1)`.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
    }

    /// Uniform integer in `[0, max)`.
    pub fn int(&mut self, max: i64) -> i64 {
        if max <= 0 {
            return 0;
        }
        (self.next() * max as f64).floor() as i64
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            return None;
        }
        items.get(self.int(items.len() as i64) as usize)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seed {
    Int(u64),
    Text(String),
}

fn hash_str(value: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in value.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

impl Seed {
    pub fn parse(s: &str) -> Self {
        if let Ok(n) = s.parse::<u64>() {
            Self::Int(n)
        } else {
            Self::Text(s.to_string())
        }
    }
}

fn unseeded() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(1);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed) as u64;
        n.wrapping_mul(0x9E37_79B9).wrapping_add(0x7F4A_7C15)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        nanos ^ (nanos << 13) ^ 0x9E37_79B9_7F4A_7C15
    }
}
