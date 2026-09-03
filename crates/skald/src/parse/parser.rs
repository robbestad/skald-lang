use crate::aliases::{resolve_arg_name, resolve_table_name};
use crate::ast::{
    BlockAlt, BlockNode, CarrierKind, EscapeNode, Node, QueryNode, TagNode, TextNode,
};
use crate::error::Error;
use crate::span::Span;

use super::lexer::{Token, TokenKind, tokenize};

pub fn decode_sep_arg(arg: &str) -> String {
    let trimmed = arg.trim();
    if matches!(trimmed, "n" | "N" | "\\n" | "\\N") {
        return "\n".to_string();
    }
    if matches!(trimmed, "s" | "S" | "\\s" | "\\S") {
        return " ".to_string();
    }
    trimmed
        .replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\s", " ")
        .replace("\\S", " ")
        .replace("\\t", "\t")
}

fn split_at_depth(input: &str, sep: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut depth_sq = 0i32;
    let mut depth_curly = 0i32;
    let mut depth_par = 0i32;
    let mut in_tick = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\\' && i + 1 < chars.len() {
            buf.push(ch);
            buf.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if ch == '`' {
            in_tick = !in_tick;
            buf.push(ch);
            i += 1;
            continue;
        }
        if !in_tick {
            match ch {
                '[' => depth_sq += 1,
                ']' => depth_sq = (depth_sq - 1).max(0),
                '{' => depth_curly += 1,
                '}' => depth_curly = (depth_curly - 1).max(0),
                '(' => depth_par += 1,
                ')' => depth_par = (depth_par - 1).max(0),
                c if c == sep && depth_sq == 0 && depth_curly == 0 && depth_par == 0 => {
                    parts.push(std::mem::take(&mut buf));
                    i += 1;
                    continue;
                }
                _ => {}
            }
        }
        buf.push(ch);
        i += 1;
    }
    parts.push(buf);
    parts
}

fn take_regex(body: &str) -> (String, Option<String>, bool) {
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let neg = bytes[i] == b'!' && i + 1 < bytes.len() && bytes[i + 1] == b'~';
        let pos = bytes[i] == b'~';
        if !neg && !pos {
            i += 1;
            continue;
        }
        let start = i;
        i += if neg { 2 } else { 1 };
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'/' {
            i += 1;
            let re_start = i;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'/' {
                    let pat = body[re_start..i].to_string();
                    let mut rebuilt = String::new();
                    rebuilt.push_str(&body[..start]);
                    rebuilt.push_str(&body[i + 1..]);
                    return (rebuilt, Some(pat), neg);
                }
                i += 1;
            }
            break;
        }
        i = start + 1;
    }
    (body.to_string(), None, false)
}

pub fn parse_query_inner(inner: &str, span: Span) -> QueryNode {
    let carrier_idx = inner.find("::");
    let (mut body, carrier, carrier_kind, extra_args) = if let Some(idx) = carrier_idx {
        let rest = inner[idx + 2..].trim();
        let (kind, ident) = if let Some(stripped) = rest.strip_prefix('~') {
            (CarrierKind::Rhyme, stripped.trim())
        } else if let Some(stripped) = rest.strip_prefix('&') {
            (CarrierKind::Rhyme, stripped.trim())
        } else if let Some(stripped) = rest.strip_prefix('!') {
            (CarrierKind::Unique, stripped.trim_start_matches('!').trim())
        } else if let Some(stripped) = rest.strip_prefix('=') {
            (CarrierKind::Match, stripped.trim())
        } else {
            (CarrierKind::Match, rest)
        };
        let mut parts = ident.split_whitespace();
        let carrier = parts.next().filter(|s| !s.is_empty()).map(str::to_string);
        let extra: Vec<String> = parts.map(resolve_arg_name).collect();
        (inner[..idx].to_string(), carrier, Some(kind), extra)
    } else {
        (inner.to_string(), None, None, Vec::new())
    };

    let (stripped, regex, regex_neg) = take_regex(&body);
    body = stripped;

    let mut exclude = Vec::new();
    let mut args = Vec::new();
    let mut table = String::new();
    let mut plural_sub = None;
    let mut i = 0usize;
    let b = body.as_bytes();
    while i < b.len() {
        if b[i] == b'.' && i + 1 < b.len() && b[i + 1] == b'.' {
            i += 2;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                let start = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                plural_sub = Some(resolve_arg_name(&body[start..i]));
            }
            continue;
        }
        if b[i].is_ascii_whitespace() || b[i] == b'.' || b[i] == b'-' {
            if b[i] == b'-' && i + 1 < b.len() && b[i + 1] == b'!' {
                i += 2;
                let start = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                if start < i {
                    exclude.push(resolve_arg_name(&body[start..i]));
                }
                continue;
            }
            i += 1;
            continue;
        }
        if b[i] == b'?' && i + 1 < b.len() && b[i + 1] == b'!' {
            i += 2;
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            if start < i {
                exclude.push(resolve_arg_name(&body[start..i]));
            }
            continue;
        }
        if b[i].is_ascii_alphanumeric() || b[i] == b'_' {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let word = &body[start..i];
            if table.is_empty() {
                table = resolve_table_name(word);
            } else {
                args.push(resolve_arg_name(word));
            }
            continue;
        }
        i += 1;
    }
    args.extend(extra_args);

    QueryNode {
        table,
        args,
        exclude,
        carrier,
        carrier_kind,
        regex,
        regex_neg,
        plural_sub,
        raw: inner.trim().to_string(),
        span,
    }
}

fn parse_replace_arg_nodes(rest: &str, span: Span) -> Result<Vec<Vec<Node>>, Error> {
    let parts = split_replace_args(rest);
    let mut args = Vec::new();
    if let Some(input) = parts.first() {
        args.push(parse(input)?);
    }
    if let Some(pat) = parts.get(1) {
        args.push(vec![Node::Text(TextNode {
            value: pat.clone(),
            span,
        })]);
    }
    if let Some(body) = parts.get(2) {
        args.push(parse(body)?);
    }
    Ok(args)
}

/// `[replace: input; /pat/; body]` — input may contain `;`; the regex is `/…/`.
fn split_replace_args(rest: &str) -> Vec<String> {
    if let Some((before, pat, after)) = find_slash_regex(rest) {
        let input = before
            .trim_end()
            .strip_suffix(';')
            .unwrap_or(before.trim_end())
            .to_string();
        let body = after.strip_prefix(';').unwrap_or(after).to_string();
        return vec![input, pat, body];
    }
    split_at_depth(rest, ';')
}

fn find_slash_regex(s: &str) -> Option<(&str, String, &str)> {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut depth_sq = 0i32;
    let mut depth_curly = 0i32;
    let mut depth_par = 0i32;
    while i < bytes.len() {
        let ch = bytes[i];
        if ch == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        match ch {
            b'[' => depth_sq += 1,
            b']' => depth_sq = (depth_sq - 1).max(0),
            b'{' => depth_curly += 1,
            b'}' => depth_curly = (depth_curly - 1).max(0),
            b'(' => depth_par += 1,
            b')' => depth_par = (depth_par - 1).max(0),
            b'/' if depth_sq == 0 && depth_curly == 0 && depth_par == 0 => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'/' {
                        let pat = s[start..=i].to_string();
                        return Some((&s[..start], pat, &s[i + 1..]));
                    }
                    i += 1;
                }
                return None;
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn parse_tag_inner(inner: &str, span: Span) -> Result<TagNode, Error> {
    let trimmed = inner.trim();
    if let Some(rest) = trimmed.strip_prefix('`') {
        let end_tick = rest.find('`').unwrap_or(rest.len());
        let regex = rest[..end_tick].to_string();
        let after = if end_tick < rest.len() {
            rest[end_tick + 1..]
                .strip_prefix(':')
                .unwrap_or(&rest[end_tick + 1..])
        } else {
            ""
        };
        let raw_args = if after.is_empty() {
            Vec::new()
        } else {
            split_at_depth(after, ';')
        };
        let args = raw_args
            .iter()
            .map(|a| parse(a))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(TagNode {
            name: "replacer".to_string(),
            arg: regex,
            args,
            span,
        });
    }
    let colon = split_at_depth(trimmed, ':');
    let name = colon
        .first()
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let rest = colon.get(1..).map(|s| s.join(":")).unwrap_or_default();
    let raw_args = if rest.is_empty() {
        Vec::new()
    } else if name == "replace" {
        split_replace_args(&rest)
    } else {
        split_at_depth(&rest, ';')
    };
    let args = if name == "replace" && !rest.is_empty() {
        parse_replace_arg_nodes(&rest, span)?
    } else {
        raw_args
            .iter()
            .map(|a| parse(a))
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut arg = raw_args
        .first()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if name == "sep" || name == "s" {
        arg = decode_sep_arg(&arg);
    }
    Ok(TagNode {
        name,
        arg,
        args,
        span,
    })
}

fn extract_weight(nodes: Vec<Node>) -> Result<BlockAlt, Error> {
    let mut i = 0usize;
    while i < nodes.len() {
        if let Node::Text(t) = &nodes[i] {
            if t.value.trim().is_empty() {
                i += 1;
                continue;
            }
        }
        break;
    }
    let Some(Node::Text(first)) = nodes.get(i) else {
        return Ok(BlockAlt {
            weight: None,
            nodes,
        });
    };
    let leading_len = first
        .value
        .chars()
        .take_while(|c| c.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let v = &first.value[leading_len..];
    if !v.starts_with('(') {
        return Ok(BlockAlt {
            weight: None,
            nodes,
        });
    }
    let mut depth = 0i32;
    let mut end = None;
    for (k, ch) in v.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(k);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(end) = end else {
        return Ok(BlockAlt {
            weight: None,
            nodes,
        });
    };
    let inner = v[1..end].to_string();
    let rest_text = v[end + 1..].to_string();
    let mut rest = Vec::new();
    if !rest_text.is_empty() {
        rest.push(Node::Text(TextNode {
            value: rest_text,
            span: first.span,
        }));
    }
    rest.extend(nodes.into_iter().skip(i + 1));
    let weight = if inner.trim().is_empty() {
        None
    } else {
        Some(parse(&inner)?)
    };
    Ok(BlockAlt {
        weight,
        nodes: rest,
    })
}

struct Parser {
    tokens: Vec<Token>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        self.tokens
            .get(self.i)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn advance(&mut self) -> Token {
        let t = self.peek().clone();
        if !matches!(t.kind, TokenKind::Eof) {
            self.i += 1;
        }
        t
    }

    fn parse_pattern(&mut self) -> Result<Vec<Node>, Error> {
        self.parse_sequence(true)
    }

    fn parse_sequence(&mut self, top: bool) -> Result<Vec<Node>, Error> {
        let mut nodes = Vec::new();
        loop {
            let t = self.peek();
            match &t.kind {
                TokenKind::Eof => {
                    if !top {
                        return Err(Error::parse("Unclosed block", t.span));
                    }
                    break;
                }
                TokenKind::RBrace | TokenKind::Pipe => {
                    if top {
                        let t = self.advance();
                        let value = if matches!(t.kind, TokenKind::RBrace) {
                            "}"
                        } else {
                            "|"
                        };
                        nodes.push(Node::Text(TextNode {
                            value: value.to_string(),
                            span: t.span,
                        }));
                        continue;
                    }
                    break;
                }
                _ => nodes.push(self.parse_node()?),
            }
        }
        Ok(nodes)
    }

    fn parse_node(&mut self) -> Result<Node, Error> {
        let t = self.advance();
        match t.kind {
            TokenKind::Text(value) => Ok(Node::Text(TextNode {
                value,
                span: t.span,
            })),
            TokenKind::Query(inner) => Ok(Node::Query(parse_query_inner(&inner, t.span))),
            TokenKind::Tag(inner) => Ok(Node::Tag(parse_tag_inner(&inner, t.span)?)),
            TokenKind::Escape(code) => Ok(Node::Escape(EscapeNode { code, span: t.span })),
            TokenKind::LBrace => self.parse_block(t.span),
            other => Err(Error::parse(format!("Unexpected token {other:?}"), t.span)),
        }
    }

    fn parse_block(&mut self, start: Span) -> Result<Node, Error> {
        let mut alternatives = Vec::new();
        loop {
            alternatives.push(extract_weight(self.parse_sequence(false)?)?);
            let t = self.peek();
            match t.kind {
                TokenKind::Pipe => {
                    self.advance();
                    continue;
                }
                TokenKind::RBrace => {
                    let end = self.advance();
                    return Ok(Node::Block(BlockNode {
                        alternatives,
                        span: Span::new(start.start as usize, end.span.end as usize),
                    }));
                }
                _ => {
                    return Err(Error::parse("Unclosed block", t.span));
                }
            }
        }
    }
}

pub fn parse(input: &str) -> Result<Vec<Node>, Error> {
    let tokens = tokenize(input);
    Parser { tokens, i: 0 }.parse_pattern()
}
