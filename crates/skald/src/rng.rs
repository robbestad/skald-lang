const MULTIPLIER: u64 = 6364136223846793005;
const INCREMENT: u64 = 1442695040888963407;

/// Named run profile: PCG32, FNV-1a text-seed hashing, left-to-right candidate order.
pub const RUN_PROFILE: &str = "skald-pcg32-v1";
const TEXT_SEED_PREFIX: &str = "text:";

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
    /// Parse a canonical seed string.
    ///
    /// - `0` or `[1-9][0-9]*` that fits in `u64` → `Int`
    /// - `text:<value>` → `Text` (explicit type, even when `<value>` is digits)
    /// - any other non-numeric string → `Text`
    ///
    /// Empty strings, leading zeros, signs, fractions, exponents, and u64 overflow
    /// are errors. `"42"` and `42` are the same integer seed; `"042"` is not.
    pub fn parse(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            return Err("seed must not be empty".to_string());
        }
        let trimmed = s.trim();
        if trimmed != s && (is_canonical_u64_decimal(trimmed) || looks_numeric(trimmed)) {
            return Err(format!(
                "invalid integer seed {s:?}; surrounding whitespace is not part of a u64 decimal"
            ));
        }
        if let Some(rest) = s.strip_prefix(TEXT_SEED_PREFIX) {
            if rest.is_empty() {
                return Err("text seed must not be empty".to_string());
            }
            return Ok(Self::Text(rest.to_string()));
        }
        if is_canonical_u64_decimal(s) {
            return s
                .parse::<u64>()
                .map(Self::Int)
                .map_err(|_| format!("integer seed {s:?} does not fit in u64"));
        }
        if looks_numeric(s) {
            return Err(format!(
                "invalid integer seed {s:?}; use a canonical u64 decimal (no sign, fraction, exponent, or leading zeros) or a non-numeric text seed"
            ));
        }
        Ok(Self::Text(s.to_string()))
    }

    pub fn encode(&self) -> String {
        match self {
            Self::Int(n) => n.to_string(),
            Self::Text(s) => format!("{TEXT_SEED_PREFIX}{s}"),
        }
    }
}

fn is_canonical_u64_decimal(s: &str) -> bool {
    if s == "0" {
        return true;
    }
    let mut chars = s.chars();
    match chars.next() {
        Some('1'..='9') => chars.all(|c| c.is_ascii_digit()),
        _ => false,
    }
}

fn looks_numeric(s: &str) -> bool {
    let rest = s.strip_prefix(['+', '-']).unwrap_or(s);
    if rest.is_empty() {
        return false;
    }
    let bytes = rest.as_bytes();
    let mut i = 0;
    let mut seen_digit = false;
    let mut seen_dot = false;
    while i < bytes.len() {
        match bytes[i] {
            b'0'..=b'9' => {
                seen_digit = true;
                i += 1;
            }
            b'.' if !seen_dot => {
                seen_dot = true;
                i += 1;
            }
            b'e' | b'E' if seen_digit => {
                i += 1;
                if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
                let exp = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                return seen_digit && i == bytes.len() && i > exp;
            }
            _ => return false,
        }
    }
    seen_digit
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
