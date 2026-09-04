const AN_WORDS: &[&str] = &["hour", "honest", "honor", "honour", "heir", "aunts", "aunt"];
const A_WORDS: &[&str] = &[
    "university",
    "user",
    "union",
    "one",
    "once",
    "european",
    "ewe",
    "uber",
];

pub fn indefinite_article(word: &str) -> &'static str {
    let first = word.split_whitespace().next().unwrap_or("");
    let lower: String = first
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .flat_map(|c| c.to_lowercase())
        .collect();
    if lower.is_empty() {
        return "a";
    }
    if AN_WORDS.contains(&lower.as_str()) {
        return "an";
    }
    if A_WORDS.contains(&lower.as_str()) {
        return "a";
    }
    match lower.chars().next() {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

pub fn with_article(word: &str) -> String {
    let trimmed = word.trim_start();
    if trimmed.is_empty() {
        return word.to_string();
    }
    format!("{} {trimmed}", indefinite_article(trimmed))
}
