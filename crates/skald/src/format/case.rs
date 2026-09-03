use crate::ast::CaseMode;

const TITLE_SMALL: &[&str] = &[
    "a", "an", "the", "and", "but", "or", "for", "nor", "as", "at", "by", "from", "in", "into",
    "near", "of", "on", "onto", "to", "with",
];
const TITLE_UPPER: &[&str] = &["id", "tv", "lsd"];

fn capitalize_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut out = first.to_uppercase().collect::<String>();
            out.extend(chars.flat_map(|c| c.to_lowercase()));
            out
        }
    }
}

pub fn apply_case(input: &str, mode: CaseMode) -> String {
    if input.is_empty() {
        return String::new();
    }
    match mode {
        CaseMode::None => input.to_string(),
        CaseMode::Upper => input.to_uppercase(),
        CaseMode::Lower => input.to_lowercase(),
        CaseMode::Word => word_case(input),
        CaseMode::Title => title_case(input),
        CaseMode::Sentence => sentence_case(input),
        CaseMode::First | CaseMode::Default => first_case(input),
    }
}

fn word_case(input: &str) -> String {
    let mut out = String::new();
    let mut start = 0usize;
    for (i, ch) in input.char_indices() {
        if ch.is_whitespace() {
            if start < i {
                out.push_str(&capitalize_word(&input[start..i]));
            }
            out.push(ch);
            start = i + ch.len_utf8();
        }
    }
    if start < input.len() {
        out.push_str(&capitalize_word(&input[start..]));
    }
    out
}

fn title_case(input: &str) -> String {
    let parts: Vec<&str> = split_keep_ws(input);
    let word_indexes: Vec<usize> = parts
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.is_empty() && !p.chars().all(char::is_whitespace))
        .map(|(i, _)| i)
        .collect();
    let first = word_indexes.first().copied();
    let last = word_indexes.last().copied();
    parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            if part.is_empty() || part.chars().all(char::is_whitespace) {
                return (*part).to_string();
            }
            let lower = part.to_lowercase();
            if TITLE_UPPER.contains(&lower.as_str()) {
                return part.to_uppercase();
            }
            let is_edge = Some(i) == first || Some(i) == last;
            if !is_edge && TITLE_SMALL.contains(&lower.as_str()) {
                return lower;
            }
            capitalize_word(part)
        })
        .collect()
}

fn split_keep_ws(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut in_ws: Option<bool> = None;
    for (i, ch) in input.char_indices() {
        let ws = ch.is_whitespace();
        match in_ws {
            None => in_ws = Some(ws),
            Some(prev) if prev != ws => {
                parts.push(&input[start..i]);
                start = i;
                in_ws = Some(ws);
            }
            _ => {}
        }
    }
    if start < input.len() || input.is_empty() {
        parts.push(&input[start..]);
    }
    parts
}

fn sentence_case(input: &str) -> String {
    let mut out = String::new();
    let mut cap_next = true;
    for ch in input.chars() {
        if cap_next && ch.is_alphanumeric() {
            out.extend(ch.to_uppercase());
            cap_next = false;
        } else {
            out.push(ch);
            if matches!(ch, '.' | '!' | '?') {
                cap_next = true;
            }
        }
    }
    out
}

fn first_case(input: &str) -> String {
    let mut chars = input.chars();
    let mut out = String::new();
    for ch in chars.by_ref() {
        if ch.is_whitespace() {
            out.push(ch);
            continue;
        }
        out.extend(ch.to_uppercase());
        break;
    }
    out.extend(chars);
    out
}

pub fn parse_case_mode(arg: &str) -> CaseMode {
    match arg.trim().to_ascii_lowercase().as_str() {
        "none" => CaseMode::None,
        "default" => CaseMode::Default,
        "first" => CaseMode::First,
        "word" => CaseMode::Word,
        "title" => CaseMode::Title,
        "upper" => CaseMode::Upper,
        "lower" => CaseMode::Lower,
        "sentence" => CaseMode::Sentence,
        "case" => CaseMode::Default,
        _ => CaseMode::Default,
    }
}
