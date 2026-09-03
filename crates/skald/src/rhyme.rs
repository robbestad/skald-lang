//! Query rhyme: perfect / slant / alliteration over Rantionary SAMPA phones.

use std::collections::HashSet;

/// Rant 3 vowel-sound symbols (X-SAMPA subset used in `.dic` `| pron` lines).
const VOWEL_SOUNDS: &[char] = &['A', 'i', 'I', 'E', 'e', '3', '{', 'V', 'O', 'U', 'u', '^'];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RhymeMode {
    #[default]
    Perfect,
    Slant,
    Alliteration,
    /// Last vowel sound only.
    Weak,
    /// Last syllable (after the final `-` marker).
    Syllabic,
}

impl RhymeMode {
    pub fn names() -> &'static [&'static str] {
        &["perfect", "slant", "alliteration", "weak", "syllabic"]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Perfect => "perfect",
            Self::Slant => "slant",
            Self::Alliteration => "alliteration",
            Self::Weak => "weak",
            Self::Syllabic => "syllabic",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "" | "perfect" => Some(Self::Perfect),
            "slant" | "slant-rhyme" | "slantrhyme" => Some(Self::Slant),
            "alliteration" => Some(Self::Alliteration),
            "weak" | "weak-rhyme" | "weakrhyme" => Some(Self::Weak),
            "syllabic" | "syllable" => Some(Self::Syllabic),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RhymeGroup {
    pub phones: String,
    pub used: HashSet<(String, String)>,
    pub seed_word: String,
}

pub fn rhymes(mode: RhymeMode, a: &str, b: &str) -> bool {
    match (rhyme_key(mode, a), rhyme_key(mode, b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

pub fn rhyme_key(mode: RhymeMode, phones: &str) -> Option<String> {
    if phones.is_empty() {
        return None;
    }
    match mode {
        RhymeMode::Perfect => perfect_key(phones),
        RhymeMode::Slant => Some(slant_key(phones)),
        RhymeMode::Alliteration => Some(alliteration_key(phones)),
        RhymeMode::Weak => weak_key(phones),
        RhymeMode::Syllabic => Some(syllabic_key(phones)),
    }
}

/// Everything after the first stressed vowel, matching Rant 3 `RhymeFlags.Perfect`.
fn perfect_key(phones: &str) -> Option<String> {
    let i = phones.find('"')?;
    let stripped = phones[i..].replace('-', "");
    Some(from_first_vowel(&stripped).to_string())
}

/// Ending consonants after the last vowel sound (phonetic slant / half-rhyme).
fn slant_key(phones: &str) -> String {
    let mut after_vowel = None;
    for (i, c) in phones.char_indices() {
        if is_vowel_sound(c) {
            after_vowel = Some(i + c.len_utf8());
        }
    }
    let tail = match after_vowel {
        Some(i) => &phones[i..],
        None => phones,
    };
    tail.chars().filter(|c| !is_marker(*c)).collect()
}

/// Last vowel sound, ignoring stress and coda.
fn weak_key(phones: &str) -> Option<String> {
    phones
        .chars()
        .filter(|c| is_vowel_sound(*c))
        .last()
        .map(|c| c.to_string())
}

/// Phones after the last syllable break, from the first vowel.
fn syllabic_key(phones: &str) -> String {
    let last = phones.rsplit('-').next().unwrap_or(phones);
    let stripped: String = last.chars().filter(|c| !is_marker(*c)).collect();
    from_first_vowel(&stripped).to_string()
}

/// Extra X-SAMPA pronunciations: `word phones` per line (`#` comments).
pub fn parse_pron_sidecar(src: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for line in src.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((word, phones)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let word = word.trim().to_ascii_lowercase();
        let phones = phones.trim();
        if word.is_empty() || phones.is_empty() {
            continue;
        }
        map.insert(word, phones.to_string());
    }
    map
}

/// Consonants up to the first vowel sound.
fn alliteration_key(phones: &str) -> String {
    let mut out = String::new();
    for c in phones.chars() {
        if is_vowel_sound(c) {
            break;
        }
        if is_marker(c) {
            continue;
        }
        out.push(c);
    }
    out
}

fn from_first_vowel(pron: &str) -> &str {
    for (i, c) in pron.char_indices() {
        if is_vowel_sound(c) {
            return &pron[i..];
        }
    }
    &pron[pron.len()..]
}

fn is_vowel_sound(c: char) -> bool {
    VOWEL_SOUNDS.contains(&c)
}

fn is_marker(c: char) -> bool {
    matches!(c, '"' | '-' | '%' | ' ' | '`')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_picky_icky() {
        assert_eq!(
            rhyme_key(RhymeMode::Perfect, r#"p"I-ki"#).unwrap(),
            rhyme_key(RhymeMode::Perfect, r#""I-ki"#).unwrap()
        );
    }

    #[test]
    fn perfect_cat_bat_not_dog() {
        assert!(rhymes(RhymeMode::Perfect, r#"k"{t"#, r#"b"{t"#));
        assert!(!rhymes(RhymeMode::Perfect, r#"k"{t"#, r#"d"Og"#));
    }

    #[test]
    fn slant_cat_net_not_dog() {
        assert!(rhymes(RhymeMode::Slant, r#"k"{t"#, r#"n"Et"#));
        assert!(!rhymes(RhymeMode::Slant, r#"k"{t"#, r#"d"Og"#));
    }

    #[test]
    fn alliteration_dog_dude() {
        assert!(rhymes(RhymeMode::Alliteration, r#"d"Og"#, r#"d"ud"#));
        assert!(!rhymes(RhymeMode::Alliteration, r#"d"Og"#, r#"k"{t"#));
    }

    #[test]
    fn perfect_needs_stress() {
        assert!(rhyme_key(RhymeMode::Perfect, "k{t").is_none());
    }

    #[test]
    fn weak_matches_last_vowel() {
        assert!(rhymes(RhymeMode::Weak, r#"k"{t"#, r#"b"{t"#));
        assert!(rhymes(RhymeMode::Weak, r#"k"{t"#, r#"k"{n"#));
        assert!(!rhymes(RhymeMode::Weak, r#"k"{t"#, r#"d"Og"#));
    }

    #[test]
    fn syllabic_uses_last_chunk() {
        assert!(rhymes(RhymeMode::Syllabic, r#"p"I-ki"#, r#"st"I-ki"#));
        assert!(!rhymes(RhymeMode::Syllabic, r#"p"I-ki"#, r#"p"I-k{t"#));
    }

    #[test]
    fn sidecar_parses_word_phones() {
        let map = parse_pron_sidecar("# comment\ncapybara k\"{p-i-bArr-V\n");
        assert_eq!(
            map.get("capybara").map(String::as_str),
            Some(r#"k"{p-i-bArr-V"#)
        );
    }
}
