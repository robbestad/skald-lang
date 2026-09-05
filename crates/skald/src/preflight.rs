//! Static checks against parser, dictionary, and capabilities.
//! Artifact `verify`/`run` fail here. Legacy English `skald()` still emits
//! unresolved queries unless a language-pack profile is bound.

use crate::aliases::{resolve_arg_name, resolve_table_name};
use crate::ast::{BlockNode, CarrierKind, Node, QueryNode, TagNode};
use crate::dict::{Capabilities, Dictionary};
use crate::error::Error;
use crate::parse::parse;
use crate::span::Span;
use std::collections::{HashMap, HashSet};

pub fn preflight_errors(
    pattern: &str,
    dict: &Dictionary,
    capabilities: Option<&Capabilities>,
    nsfw: bool,
) -> Result<(), Error> {
    let ast = parse(pattern)?;
    let mut bound = HashMap::new();
    let mut uncertain = HashSet::new();
    walk(
        &ast,
        dict,
        capabilities,
        nsfw,
        &mut bound,
        &mut uncertain,
        false,
    )
}

fn fail(code: &str, message: String, span: Span) -> Error {
    Error::runtime(format!("{code}: {message}"), Some(span))
}

fn is_ws_text(node: &Node) -> bool {
    matches!(node, Node::Text(t) if t.value.chars().all(char::is_whitespace))
}

fn walk(
    nodes: &[Node],
    dict: &Dictionary,
    caps: Option<&Capabilities>,
    nsfw: bool,
    bound: &mut HashMap<String, String>,
    uncertain: &mut HashSet<String>,
    defer_unbound: bool,
) -> Result<(), Error> {
    let mut i = 0usize;
    while i < nodes.len() {
        match &nodes[i] {
            Node::Query(q) => {
                check_query(q, dict, caps, nsfw, bound, uncertain, defer_unbound)?;
                i += 1;
            }
            Node::Tag(t) => {
                check_tag(t, caps)?;
                let name = t.name.trim_start_matches(':');
                if name != "fn" {
                    for arg in &t.args {
                        walk(arg, dict, caps, nsfw, bound, uncertain, defer_unbound)?;
                    }
                }
                i += 1;
                if name == "fn" {
                    // `[fn:name]{body}` stores the following block; it is not linear text.
                    while i < nodes.len() && is_ws_text(&nodes[i]) {
                        i += 1;
                    }
                    if let Some(Node::Block(b)) = nodes.get(i) {
                        let mut body_bound = bound.clone();
                        let mut body_uncertain = uncertain.clone();
                        walk_block(
                            b,
                            dict,
                            caps,
                            nsfw,
                            &mut body_bound,
                            &mut body_uncertain,
                            true,
                        )?;
                        i += 1;
                    }
                }
            }
            Node::Block(b) => {
                walk_block(b, dict, caps, nsfw, bound, uncertain, defer_unbound)?;
                i += 1;
            }
            Node::Text(_) | Node::Escape(_) => i += 1,
        }
    }
    Ok(())
}

fn walk_block(
    b: &BlockNode,
    dict: &Dictionary,
    caps: Option<&Capabilities>,
    nsfw: bool,
    bound: &mut HashMap<String, String>,
    uncertain: &mut HashSet<String>,
    defer_unbound: bool,
) -> Result<(), Error> {
    let parent_bound = bound.clone();
    let parent_uncertain = uncertain.clone();
    let mut definite: Option<HashMap<String, String>> = None;
    let mut maybe = HashSet::new();
    for alt in &b.alternatives {
        let mut alt_bound = parent_bound.clone();
        let mut alt_uncertain = parent_uncertain.clone();
        if let Some(weight) = &alt.weight {
            walk(
                weight,
                dict,
                caps,
                nsfw,
                &mut alt_bound,
                &mut alt_uncertain,
                defer_unbound,
            )?;
        }
        walk(
            &alt.nodes,
            dict,
            caps,
            nsfw,
            &mut alt_bound,
            &mut alt_uncertain,
            defer_unbound,
        )?;
        let new: HashMap<String, String> = alt_bound
            .into_iter()
            .filter(|(k, _)| !parent_bound.contains_key(k))
            .collect();
        maybe.extend(new.keys().cloned());
        maybe.extend(
            alt_uncertain
                .into_iter()
                .filter(|k| !parent_uncertain.contains(k)),
        );
        definite = Some(match definite.take() {
            None => new,
            Some(mut prev) => {
                prev.retain(|k, table| new.get(k) == Some(table));
                prev
            }
        });
    }
    if let Some(definite) = definite {
        for (id, table) in definite {
            maybe.remove(&id);
            bound.insert(id, table);
        }
    }
    for id in maybe {
        if !bound.contains_key(&id) {
            uncertain.insert(id);
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
    nsfw: bool,
    bound: &mut HashMap<String, String>,
    uncertain: &mut HashSet<String>,
    defer_unbound: bool,
) -> Result<(), Error> {
    if query.carrier_kind == Some(CarrierKind::Rhyme) && caps.is_some_and(|c| !c.allows_rhyme()) {
        return Err(fail(
            "PREFLIGHT_CAPABILITY",
            "rhyme is not supported by this language pack".into(),
            query.span,
        ));
    }

    if let Some(id) = &query.carrier {
        match query.carrier_kind {
            Some(CarrierKind::Rhyme) | Some(CarrierKind::Unique) => {}
            Some(CarrierKind::Match) | None => {
                if !query.table.is_empty() {
                    bound.insert(id.clone(), query.table.clone());
                    uncertain.remove(id);
                } else if bound.contains_key(id) {
                    if let Some(table_name) = bound.get(id) {
                        if let Some(table) = dict.table(table_name) {
                            check_form_args(query, table, true)?;
                        }
                    }
                    return Ok(());
                } else if uncertain.contains(id) || defer_unbound {
                    return Ok(());
                } else {
                    return Err(fail(
                        "PREFLIGHT_UNBOUND_CARRIER",
                        format!("unbound carrier `{id}`"),
                        query.span,
                    ));
                }
            }
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

    check_form_args(query, table, false)?;
    let (form, classes) =
        crate::query::form_index(&table.subs, &query.args, query.plural_sub.as_deref(), None);
    let idxs = crate::query::select_indices(table, &classes, &query.exclude, nsfw);
    if idxs.is_empty() {
        return Err(fail(
            "PREFLIGHT_EMPTY_CANDIDATES",
            format!("empty candidate set for table `{}`", table.name),
            query.span,
        ));
    }
    if let Some(pat) = &query.regex {
        let filtered =
            crate::query::apply_regex(table, idxs, form, pat, query.regex_neg, query.span)?;
        if filtered.is_empty() {
            if query.plural_sub.is_some() {
                // Plural form depends on runtime last_number; leave emptiness to runtime.
                return Ok(());
            }
            return Err(fail(
                "PREFLIGHT_EMPTY_CANDIDATES",
                format!("empty candidate set for table `{}`", table.name),
                query.span,
            ));
        }
    }
    Ok(())
}

fn check_form_args(
    query: &QueryNode,
    table: &crate::dict::Table,
    recall: bool,
) -> Result<(), Error> {
    for raw in &query.args {
        let arg = resolve_arg_name(raw);
        if table.subs.iter().any(|s| s == &arg) || arg == "nsfw" {
            continue;
        }
        if !recall && table.by_class.contains_key(&arg) {
            continue;
        }
        return Err(fail(
            "PREFLIGHT_UNKNOWN_FORM",
            format!("unknown form or class `{arg}` on table `{}`", table.name),
            query.span,
        ));
    }
    if let Some(pl) = &query.plural_sub {
        let arg = resolve_arg_name(pl);
        if !table.subs.iter().any(|s| s == &arg) {
            return Err(fail(
                "PREFLIGHT_UNKNOWN_FORM",
                format!("unknown form `{arg}` on table `{}`", table.name),
                query.span,
            ));
        }
    }
    Ok(())
}
