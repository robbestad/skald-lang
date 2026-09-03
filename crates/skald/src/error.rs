use crate::span::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Parse { message: String, span: Span },
    Runtime { message: String, span: Option<Span> },
    Budget { message: String },
}

impl Error {
    pub fn parse(message: impl Into<String>, span: Span) -> Self {
        Self::Parse {
            message: message.into(),
            span,
        }
    }

    pub fn runtime(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::Runtime {
            message: message.into(),
            span,
        }
    }

    pub fn budget(message: impl Into<String>) -> Self {
        Self::Budget {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { message, span } => {
                write!(f, "parse error at {}..{}: {message}", span.start, span.end)
            }
            Self::Runtime { message, span } => match span {
                Some(span) => {
                    write!(
                        f,
                        "runtime error at {}..{}: {message}",
                        span.start, span.end
                    )
                }
                None => write!(f, "runtime error: {message}"),
            },
            Self::Budget { message } => write!(f, "budget exceeded: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// Closest candidate by Levenshtein distance, if near enough to be useful.
pub fn did_you_mean<'a>(got: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let got = got.trim();
    if got.is_empty() {
        return None;
    }
    let lower = got.to_ascii_lowercase();
    let max = if lower.len() <= 2 { 1 } else { 2 };
    let mut best: Option<(&'a str, usize)> = None;
    for &cand in candidates {
        let cl = cand.to_ascii_lowercase();
        if cl == lower {
            continue;
        }
        let d = levenshtein(&lower, &cl);
        if d > max {
            continue;
        }
        match best {
            None => best = Some((cand, d)),
            Some((name, bd)) if d < bd || (d == bd && cand < name) => best = Some((cand, d)),
            _ => {}
        }
    }
    best.map(|(name, _)| name)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0; m + 1];
    for i in 1..=n {
        cur[0] = i;
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m]
}

#[cfg(test)]
mod tests {
    use super::did_you_mean;

    #[test]
    fn suggests_case_for_cae() {
        assert_eq!(
            did_you_mean("cae", &["case", "caps", "chance"]),
            Some("case")
        );
    }

    #[test]
    fn no_suggestion_when_far() {
        assert_eq!(did_you_mean("zzzzzz", &["case", "let", "fn"]), None);
    }
}
