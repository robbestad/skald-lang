use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Text(String),
    Query(String),
    Tag(String),
    LBrace,
    RBrace,
    Pipe,
    Escape(String),
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut i = 0usize;
    let mut text = String::new();
    let mut text_start = 0usize;

    let flush_text = |tokens: &mut Vec<Token>, text: &mut String, start: usize, end: usize| {
        if !text.is_empty() {
            tokens.push(Token {
                kind: TokenKind::Text(std::mem::take(text)),
                span: Span::new(start, end),
            });
        }
    };

    while i < input.len() {
        let ch = input[i..].chars().next().unwrap();
        let ch_len = ch.len_utf8();

        if ch == '#' && !text.ends_with('\\') {
            flush_text(&mut tokens, &mut text, text_start, i);
            while i < input.len() && input.as_bytes()[i] != b'\n' {
                i += 1;
            }
            text_start = i;
            continue;
        }

        if ch == '\\' && i + 1 < input.len() {
            let next = input[i + 1..].chars().next().unwrap();
            let next_len = next.len_utf8();
            let start = i;
            i += ch_len + next_len;
            if next == 'C' || next == 'd' {
                flush_text(&mut tokens, &mut text, text_start, start);
                tokens.push(Token {
                    kind: TokenKind::Escape(next.to_string()),
                    span: Span::new(start, i),
                });
                text_start = i;
            } else {
                if text.is_empty() {
                    text_start = start;
                }
                text.push_str(&unescape_char(next));
            }
            continue;
        }

        if ch == '<' {
            flush_text(&mut tokens, &mut text, text_start, i);
            let start = i;
            i += ch_len;
            let inner = read_until(input, &mut i, '>');
            tokens.push(Token {
                kind: TokenKind::Query(inner),
                span: Span::new(start, i),
            });
            text_start = i;
            continue;
        }

        if ch == '[' {
            flush_text(&mut tokens, &mut text, text_start, i);
            let start = i;
            i += ch_len;
            let inner = read_tag(input, &mut i);
            tokens.push(Token {
                kind: TokenKind::Tag(inner),
                span: Span::new(start, i),
            });
            text_start = i;
            continue;
        }

        if ch == '{' {
            flush_text(&mut tokens, &mut text, text_start, i);
            tokens.push(Token {
                kind: TokenKind::LBrace,
                span: Span::new(i, i + ch_len),
            });
            i += ch_len;
            text_start = i;
            continue;
        }

        if ch == '}' {
            flush_text(&mut tokens, &mut text, text_start, i);
            tokens.push(Token {
                kind: TokenKind::RBrace,
                span: Span::new(i, i + ch_len),
            });
            i += ch_len;
            text_start = i;
            continue;
        }

        if ch == '|' {
            flush_text(&mut tokens, &mut text, text_start, i);
            tokens.push(Token {
                kind: TokenKind::Pipe,
                span: Span::new(i, i + ch_len),
            });
            i += ch_len;
            text_start = i;
            continue;
        }

        if text.is_empty() {
            text_start = i;
        }
        text.push(ch);
        i += ch_len;
    }

    flush_text(&mut tokens, &mut text, text_start, i);
    tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span::new(i, i),
    });
    tokens
}

fn unescape_char(ch: char) -> String {
    match ch {
        'n' | 'N' => "\n".to_string(),
        's' | 'S' => " ".to_string(),
        't' => "\t".to_string(),
        _ => ch.to_string(),
    }
}

fn read_until(input: &str, i: &mut usize, end: char) -> String {
    let mut out = String::new();
    while *i < input.len() {
        let ch = input[*i..].chars().next().unwrap();
        let ch_len = ch.len_utf8();
        if ch == '\\' && *i + ch_len < input.len() {
            let next = input[*i + ch_len..].chars().next().unwrap();
            out.push(next);
            *i += ch_len + next.len_utf8();
            continue;
        }
        if ch == end {
            *i += ch_len;
            return out;
        }
        out.push(ch);
        *i += ch_len;
    }
    out
}

fn read_tag(input: &str, i: &mut usize) -> String {
    let mut out = String::new();
    let mut depth = 1i32;
    let mut in_tick = false;
    while *i < input.len() && depth > 0 {
        let ch = input[*i..].chars().next().unwrap();
        let ch_len = ch.len_utf8();
        if ch == '\\' && *i + ch_len < input.len() {
            let next = input[*i + ch_len..].chars().next().unwrap();
            out.push(ch);
            out.push(next);
            *i += ch_len + next.len_utf8();
            continue;
        }
        if ch == '`' {
            in_tick = !in_tick;
            out.push(ch);
            *i += ch_len;
            continue;
        }
        if !in_tick {
            if ch == '[' {
                depth += 1;
            } else if ch == ']' {
                depth -= 1;
                *i += ch_len;
                if depth == 0 {
                    return out;
                }
                out.push(ch);
                continue;
            }
        }
        out.push(ch);
        *i += ch_len;
    }
    out
}
