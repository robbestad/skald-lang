//! Story-mode lint: flag Mad Libs query combinations in a beat.
//! Does not depend on seed or dictionary rows.

use crate::aliases::{resolve_arg_name, resolve_table_name};
use crate::ast::{Node, QueryNode};

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

pub fn lint_story(pattern: &str, ast: &[Node]) -> Vec<String> {
    let beats = beat_ranges(pattern);
    let mut queries = Vec::new();
    collect_queries(ast, &mut queries);
    let mut notes = Vec::new();
    for (i, &(start, end)) in beats.iter().enumerate() {
        let beat_n = i + 1;
        let in_beat: Vec<&QueryNode> = queries
            .iter()
            .copied()
            .filter(|q| q.span.start >= start && q.span.start < end)
            .collect();
        notes.extend(lint_beat(beat_n, &in_beat));
    }
    notes
}

fn lint_beat(beat_n: usize, queries: &[&QueryNode]) -> Vec<String> {
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

    if kinds.contains(&Kind::Verb) && has_noun {
        notes.push(format!(
            "story: beat {beat_n} combines <verb.ed> with a noun query; write the predicate as glue or a small {{…}} block"
        ));
    }
    if kinds.contains(&Kind::Adj) && has_personish {
        notes.push(format!(
            "story: beat {beat_n} uses <adj> on a person or noun; prefer a tiny {{tired|silent}} block"
        ));
    }
    if kinds.contains(&Kind::Place) {
        notes.push(format!(
            "story: beat {beat_n} uses <place> as a specific setting; write “the inn” or use a supplied scene palette"
        ));
    }
    if kinds.contains(&Kind::NounContainer) {
        notes.push(format!(
            "story: beat {beat_n} uses <noun-container> as if it were a cup; write the object or a small {{a cup|a bowl|a plate}} block"
        ));
    }
    if kinds.contains(&Kind::NounLiquid) {
        notes.push(format!(
            "story: beat {beat_n} uses <noun-liquid> as if it were a drink; write {{ale|stew|bread}} or a scene palette"
        ));
    }
    if kinds.contains(&Kind::NounSurface) {
        notes.push(format!(
            "story: beat {beat_n} uses <noun-surface> as if it were a table; write the surface in glue"
        ));
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
                let end = (i + 1) as u32;
                if end > start {
                    ranges.push((start, end));
                }
                start = end;
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
    fn newline_and_period_split_beats() {
        let r = beat_ranges("a.\nb!");
        assert_eq!(r.len(), 3);
    }
}
