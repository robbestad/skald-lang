//! Story-mode lint: flag Mad Libs query combinations in a beat.
//! Does not depend on seed or dictionary rows.

use crate::aliases::{resolve_arg_name, resolve_table_name};
use crate::ast::{Node, QueryNode};
use crate::output::Diagnostic;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Safe,
    Verb,
    Adj,
    Noun,
    Place,
    NounContainer,
    NounLiquid,
    NounSurface,
}

pub fn lint_story(pattern: &str, ast: &[Node]) -> Vec<Diagnostic> {
    let beats = beat_ranges(pattern);
    let mut queries = Vec::new();
    collect_queries(ast, &mut queries);
    let mut notes = Vec::new();
    for (i, &(start, end)) in beats.iter().enumerate() {
        let in_beat: Vec<&QueryNode> = queries
            .iter()
            .copied()
            .filter(|q| q.span.start >= start && q.span.start < end)
            .collect();
        notes.extend(lint_beat(i, &in_beat));
    }
    notes
}

fn lint_beat(beat_index: usize, queries: &[&QueryNode]) -> Vec<Diagnostic> {
    let mut notes = Vec::new();
    let kinds: Vec<Kind> = queries.iter().map(|q| kind_of(q)).collect();
    let has_noun = kinds.iter().any(|k| {
        matches!(
            k,
            Kind::Noun | Kind::NounContainer | Kind::NounLiquid | Kind::NounSurface
        )
    });
    let has_personish = queries.iter().any(|q| {
        let t = resolve_table_name(&q.table);
        t == "noun"
            && q.args
                .iter()
                .any(|a| matches!(a.as_str(), "job" | "person"))
    }) || has_noun;

    let span_of = |kind: Kind| {
        queries
            .iter()
            .zip(kinds.iter())
            .find(|(_, k)| **k == kind)
            .map(|(q, _)| q.span)
            .unwrap_or(crate::span::Span::empty())
    };

    if kinds.contains(&Kind::Verb) {
        let span = span_of(Kind::Verb);
        let message = if has_noun {
            "combines <verb.ed> with a noun query; write the predicate as glue or a small {…} block"
        } else {
            "uses an open verb query (<verb.ed>/<verb-walk>/…); write the predicate as glue or a small {…} block"
        };
        notes.push(
            Diagnostic::error("STORY_OPEN_VERB", message)
                .with_beat(beat_index, span)
                .with_hint("Write the predicate or use a small closed block"),
        );
    }
    if kinds.contains(&Kind::Adj) && has_personish {
        notes.push(
            Diagnostic::error(
                "STORY_OPEN_ADJ",
                "uses <adj> on a person or noun; prefer a tiny {tired|silent} block",
            )
            .with_beat(beat_index, span_of(Kind::Adj))
            .with_hint("Prefer a tiny {tired|silent} block"),
        );
    }
    if kinds.contains(&Kind::Place) {
        notes.push(
            Diagnostic::error(
                "STORY_OPEN_PLACE",
                "uses <place> as a specific setting; write “the inn” or use a supplied scene palette",
            )
            .with_beat(beat_index, span_of(Kind::Place))
            .with_hint("Write the setting in glue or use a scene palette"),
        );
    }
    if kinds.contains(&Kind::NounContainer) {
        notes.push(
            Diagnostic::error(
                "STORY_OPEN_CONTAINER",
                "uses <noun-container> as if it were a cup; write the object or a small {a cup|a bowl|a plate} block",
            )
            .with_beat(beat_index, span_of(Kind::NounContainer)),
        );
    }
    if kinds.contains(&Kind::NounLiquid) {
        notes.push(
            Diagnostic::error(
                "STORY_OPEN_LIQUID",
                "uses <noun-liquid> as if it were a drink; write {ale|stew|bread} or a scene palette",
            )
            .with_beat(beat_index, span_of(Kind::NounLiquid)),
        );
    }
    if kinds.contains(&Kind::NounSurface) {
        notes.push(
            Diagnostic::error(
                "STORY_OPEN_SURFACE",
                "uses <noun-surface> as if it were a table; write the surface in glue",
            )
            .with_beat(beat_index, span_of(Kind::NounSurface)),
        );
    }
    notes
}

fn kind_of(q: &QueryNode) -> Kind {
    if q.table.is_empty() {
        return Kind::Safe;
    }
    let table = resolve_table_name(&q.table);
    let args: Vec<String> = q.args.iter().map(|a| resolve_arg_name(a)).collect();
    match table.as_str() {
        "firstname" | "pron" => Kind::Safe,
        "verb" | "say" | "verbimg" => Kind::Verb,
        "adj" => Kind::Adj,
        "place" => Kind::Place,
        "noun" => {
            if args.iter().any(|a| a == "container") {
                Kind::NounContainer
            } else if args.iter().any(|a| a == "liquid") {
                Kind::NounLiquid
            } else if args.iter().any(|a| a == "surface") {
                Kind::NounSurface
            } else {
                Kind::Noun
            }
        }
        _ => Kind::Safe,
    }
}

fn collect_queries<'a>(nodes: &'a [Node], out: &mut Vec<&'a QueryNode>) {
    for node in nodes {
        match node {
            Node::Query(q) => out.push(q),
            Node::Tag(t) => {
                for arg in &t.args {
                    collect_queries(arg, out);
                }
            }
            Node::Block(b) => {
                for alt in &b.alternatives {
                    if let Some(w) = &alt.weight {
                        collect_queries(w, out);
                    }
                    collect_queries(&alt.nodes, out);
                }
            }
            Node::Text(_) | Node::Escape(_) => {}
        }
    }
}

/// Beat ranges as byte offsets. Split on newlines and on `.!?` outside `<>[]{}`.
fn beat_ranges(pattern: &str) -> Vec<(u32, u32)> {
    let bytes = pattern.as_bytes();
    if bytes.is_empty() {
        return vec![(0, 0)];
    }
    let mut ranges = Vec::new();
    let mut start = 0u32;
    let mut depth_angle = 0i32;
    let mut depth_sq = 0i32;
    let mut depth_curly = 0i32;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        match b {
            b'<' => depth_angle += 1,
            b'>' => depth_angle = (depth_angle - 1).max(0),
            b'[' => depth_sq += 1,
            b']' => depth_sq = (depth_sq - 1).max(0),
            b'{' => depth_curly += 1,
            b'}' => depth_curly = (depth_curly - 1).max(0),
            b'\n' | b'.' | b'!' | b'?' if depth_angle == 0 && depth_sq == 0 && depth_curly == 0 => {
                let mut end = i + 1;
                if bytes[i] != b'\n' {
                    while end < bytes.len() && bytes[end].is_ascii_whitespace() {
                        end += 1;
                    }
                }
                if end as u32 > start {
                    ranges.push((start, end as u32));
                }
                start = end as u32;
            }
            _ => {}
        }
        i += 1;
    }
    if start < bytes.len() as u32 {
        ranges.push((start, bytes.len() as u32));
    }
    if ranges.is_empty() {
        ranges.push((0, bytes.len() as u32));
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::beat_ranges;

    #[test]
    fn newline_after_period_is_one_split() {
        let r = beat_ranges("a.\nb!");
        assert_eq!(r.len(), 2, "{r:?}");
    }

    #[test]
    fn json_beats_keep_indexes() {
        let r = beat_ranges("one.\ntwo.\nthree.");
        assert_eq!(r.len(), 3, "{r:?}");
    }
}
