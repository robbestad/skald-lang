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
}

impl RhymeMode {
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "" | "perfect" => Some(Self::Perfect),
            "slant" | "slant-rhyme" | "slantrhyme" => Some(Self::Slant),
            "alliteration" => Some(Self::Alliteration),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RhymeGroup {
    pub phones: String,
    pub used: HashSet<(String, String)>,
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
}
