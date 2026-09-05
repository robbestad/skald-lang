//! Static checks against parser, dictionary, and capabilities.
//! Artifact `verify`/`run` fail here. Legacy English `skald()` still emits
//! unresolved queries unless a language-pack profile is bound.

use crate::aliases::{resolve_arg_name, resolve_table_name};
use crate::ast::{CarrierKind, Node, QueryNode, TagNode};
use crate::dict::{Capabilities, Dictionary};
use crate::error::Error;
use crate::parse::parse;
use crate::span::Span;
use std::collections::HashSet;

pub fn preflight_errors(
    pattern: &str,
    dict: &Dictionary,
    capabilities: Option<&Capabilities>,
) -> Result<(), Error> {
    let ast = parse(pattern)?;
    let mut bound = HashSet::new();
    walk(&ast, dict, capabilities, &mut bound)
}

fn fail(code: &str, message: String, span: Span) -> Error {
    Error::runtime(format!("{code}: {message}"), Some(span))
}

fn walk(
    nodes: &[Node],
    dict: &Dictionary,
    caps: Option<&Capabilities>,
    bound: &mut HashSet<String>,
) -> Result<(), Error> {
    for node in nodes {
        match node {
            Node::Query(q) => check_query(q, dict, caps, bound)?,
            Node::Tag(t) => {
                check_tag(t, caps)?;
                for arg in &t.args {
                    walk(arg, dict, caps, bound)?;
                }
            }
            Node::Block(b) => {
                for alt in &b.alternatives {
                    if let Some(weight) = &alt.weight {
                        walk(weight, dict, caps, bound)?;
                    }
                    walk(&alt.nodes, dict, caps, bound)?;
                }
            }
            Node::Text(_) | Node::Escape(_) => {}
        }
    }
    Ok(())
}

fn check_tag(tag: &TagNode, caps: Option<&Capabilities>) -> Result<(), Error> {
    let Some(caps) = caps else {
        return Ok(());
    };
    let name = tag.name.trim_start_matches(':');
    let unsupported = match name {
        "a" if !caps.allows_articles() => {
            Some("indefinite articles are not supported by this language pack")
        }
        "rhyme" if !caps.allows_rhyme() => Some("rhyme is not supported by this language pack"),
        "case" | "caps" => {
            let raw = if tag.arg.is_empty() {
                static_text(tag.args.first().map(|v| v.as_slice()).unwrap_or(&[]))
            } else {
                tag.arg.clone()
            };
            if raw.trim().eq_ignore_ascii_case("title") && !caps.allows_title_case() {
                Some("title case is not supported by this language pack")
            } else {
                None
            }
        }
        "numfmt" => {
            let raw = static_text(tag.args.first().map(|v| v.as_slice()).unwrap_or(&[]));
            if raw.trim().eq_ignore_ascii_case("verbal") && !caps.allows_verbal_numbers() {
                Some("verbal numbers are not supported by this language pack")
            } else {
                None
            }
        }
        _ => None,
    };
    if let Some(message) = unsupported {
        return Err(fail("PREFLIGHT_CAPABILITY", message.into(), tag.span));
    }
    Ok(())
}

fn static_text(nodes: &[Node]) -> String {
    nodes
        .iter()
        .filter_map(|n| match n {
            Node::Text(t) => Some(t.value.as_str()),
            _ => None,
        })
        .collect()
}

fn check_query(
    query: &QueryNode,
    dict: &Dictionary,
    caps: Option<&Capabilities>,
    bound: &mut HashSet<String>,
) -> Result<(), Error> {
    if query.carrier_kind == Some(CarrierKind::Rhyme) && caps.is_some_and(|c| !c.allows_rhyme()) {
        return Err(fail(
            "PREFLIGHT_CAPABILITY",
            "rhyme is not supported by this language pack".into(),
            query.span,
        ));
    }

    if let Some(id) = &query.carrier {
        let is_bind =
            !query.table.is_empty() && !matches!(query.carrier_kind, Some(CarrierKind::Rhyme));
        if is_bind {
            bound.insert(id.clone());
        } else if !bound.contains(id) {
            return Err(fail(
                "PREFLIGHT_UNBOUND_CARRIER",
                format!("unbound carrier `{id}`"),
                query.span,
            ));
        }
    }

    if query.table.is_empty() {
        return Ok(());
    }

    let Some(table) = dict.table(&query.table) else {
        return Err(fail(
            "PREFLIGHT_UNKNOWN_TABLE",
            format!("unknown table `{}`", resolve_table_name(&query.table)),
            query.span,
        ));
    };

    let mut classes = Vec::new();
    for raw in &query.args {
        let arg = resolve_arg_name(raw);
        if table.subs.iter().any(|s| s == &arg) || arg == "nsfw" {
            continue;
        }
        if table.by_class.contains_key(&arg) {
            classes.push(arg);
            continue;
        }
        return Err(fail(
            "PREFLIGHT_UNKNOWN_FORM",
            format!("unknown form or class `{arg}` on table `{}`", table.name),
            query.span,
        ));
    }
    let idxs = crate::query::select_indices(table, &classes, &query.exclude, false);
    if idxs.is_empty() {
        let detail = if classes.is_empty() {
            format!("empty candidate set for table `{}`", table.name)
        } else {
            format!(
                "empty candidate set for table `{}` with classes [{}]",
                table.name,
                classes.join(" ")
            )
        };
        return Err(fail("PREFLIGHT_EMPTY_CANDIDATES", detail, query.span));
    }
    Ok(())
}
