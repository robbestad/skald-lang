use crate::ast::{CarrierKind, QueryNode};
use crate::dict::{BoundEntry, Entry, Table};
use crate::error::Error;
use crate::output::{QueryPick, UnresolvedQuery};
use crate::rhyme::{RhymeGroup, rhymes};
use crate::runtime::Context;
use crate::value::Value;
use regex::Regex;
use std::sync::Arc;

pub struct QueryResult {
    pub text: String,
    pub entry: Option<BoundEntry>,
}

impl QueryResult {
    fn text(text: String) -> Self {
        Self { text, entry: None }
    }

    pub fn into_value(self) -> Value {
        if let Some(entry) = self.entry {
            Value::Entry(entry)
        } else if self.text.is_empty() {
            Value::Nil
        } else {
            Value::Str(self.text)
        }
    }
}

fn matches_class(entry: &Entry, cls: &str) -> bool {
    let classes = &entry.classes;
    if classes.iter().any(|c| c == cls) {
        return true;
    }
    match cls {
        "male" => classes.iter().any(|c| c == "male?"),
        "female" => classes.iter().any(|c| c == "female?"),
        "neutral" => {
            classes.iter().any(|c| c == "neutral")
                || (classes.iter().any(|c| c == "male?") && classes.iter().any(|c| c == "female?"))
        }
        _ => false,
    }
}

fn form_of(entry: &Entry, index: usize) -> &str {
    entry
        .forms
        .get(index)
        .or_else(|| entry.forms.first())
        .map(String::as_str)
        .unwrap_or("")
}

fn phones_of(entry: &Entry, index: usize) -> &str {
    entry.phones.get(index).map(String::as_str).unwrap_or("")
}

fn phones_for(entry: &Entry, form: usize, ctx: &Context) -> String {
    let direct = phones_of(entry, form);
    if !direct.is_empty() {
        return direct.to_string();
    }
    let surface = form_of(entry, form).to_ascii_lowercase();
    ctx.pronunciations
        .get(&surface)
        .cloned()
        .unwrap_or_default()
}

fn entry_ident(table: &Table, entry: &Entry) -> (String, String) {
    (
        table.name.clone(),
        entry.forms.first().cloned().unwrap_or_default(),
    )
}

fn intersect_sorted(a: &[usize], b: &[usize]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Equal => {
                out.push(a[i]);
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    out
}

pub(crate) fn form_index(
    subs: &[String],
    args: &[String],
    plural_sub: Option<&str>,
    last_number: Option<i64>,
) -> (usize, Vec<String>) {
    let mut form = 0usize;
    let mut classes = Vec::new();
    for arg in args {
        if let Some(i) = subs.iter().position(|s| s == arg) {
            form = i;
        } else {
            classes.push(arg.clone());
        }
    }
    if let (Some(pl), Some(n)) = (plural_sub, last_number) {
        if n != 1 {
            if let Some(i) = subs.iter().position(|s| s == pl) {
                form = i;
            }
        }
    }
    (form, classes)
}

pub(crate) fn select_indices(
    table: &Table,
    classes: &[String],
    exclude: &[String],
    nsfw: bool,
) -> Vec<usize> {
    let want_nsfw = nsfw || classes.iter().any(|c| c == "nsfw");
    let mut idxs: Option<Vec<usize>> = None;
    for cls in classes {
        if cls == "nsfw" {
            continue;
        }
        let Some(bucket) = table.by_class.get(cls) else {
            return Vec::new();
        };
        if bucket.is_empty() {
            return Vec::new();
        }
        idxs = Some(match idxs {
            Some(prev) => intersect_sorted(&prev, bucket),
            None => bucket.clone(),
        });
    }

    let mut list: Vec<usize> = idxs.unwrap_or_else(|| (0..table.entries.len()).collect());

    if table.has_nsfw && !want_nsfw {
        list.retain(|&i| !table.entries[i].classes.iter().any(|c| c == "nsfw"));
    }
    for cls in exclude {
        list.retain(|&i| !matches_class(&table.entries[i], cls));
    }
    list
}

pub(crate) fn apply_regex(
    table: &Table,
    idxs: Vec<usize>,
    form: usize,
    pat: &str,
    neg: bool,
    span: crate::span::Span,
) -> Result<Vec<usize>, Error> {
    let re = Regex::new(pat)
        .map_err(|e| Error::runtime(format!("invalid query regex /{pat}/: {e}"), Some(span)))?;
    Ok(idxs
        .into_iter()
        .filter(|&i| {
            let text = form_of(&table.entries[i], form);
            let hit = re.is_match(text);
            if neg { !hit } else { hit }
        })
        .collect())
}

fn idxs_with_rhyme_partners(
    table: &Table,
    idxs: &[usize],
    form: usize,
    ctx: &Context,
) -> Vec<usize> {
    let mode = ctx.rhyme_mode;
    let keys: Vec<Option<String>> = idxs
        .iter()
        .map(|&i| crate::rhyme::rhyme_key(mode, &phones_for(&table.entries[i], form, ctx)))
        .collect();
    let mut counts = std::collections::HashMap::<String, usize>::new();
    for k in keys.iter().flatten() {
        *counts.entry(k.clone()).or_default() += 1;
    }
    idxs.iter()
        .enumerate()
        .filter_map(|(n, &i)| {
            keys[n]
                .as_ref()
                .filter(|k| counts.get(*k).copied().unwrap_or(0) >= 2)
                .map(|_| i)
        })
        .collect()
}

fn apply_rhyme(
    table: &Table,
    idxs: Vec<usize>,
    form: usize,
    id: &str,
    ctx: &Context,
    span: crate::span::Span,
) -> Result<Vec<usize>, Error> {
    match ctx.rhyme_carriers.get(id) {
        None => {
            let with_phones: Vec<usize> = idxs
                .into_iter()
                .filter(|&i| !phones_for(&table.entries[i], form, ctx).is_empty())
                .collect();
            if with_phones.is_empty() {
                return Err(Error::runtime(
                    format!("rhyme carrier '{id}' needs pronunciation data"),
                    Some(span),
                ));
            }
            let paired = idxs_with_rhyme_partners(table, &with_phones, form, ctx);
            Ok(if paired.is_empty() {
                with_phones
            } else {
                paired
            })
        }
        Some(group) => {
            let mode = ctx.rhyme_mode;
            Ok(idxs
                .into_iter()
                .filter(|&i| {
                    let entry = &table.entries[i];
                    let p = phones_for(entry, form, ctx);
                    if p.is_empty() {
                        return false;
                    }
                    !group.used.contains(&entry_ident(table, entry))
                        && rhymes(mode, &group.phones, &p)
                })
                .collect())
        }
    }
}

pub fn resolve_query(query: &QueryNode, ctx: &mut Context) -> Result<QueryResult, Error> {
    let is_match =
        matches!(query.carrier_kind, Some(CarrierKind::Match) | None) && query.carrier.is_some();

    if is_match {
        if let Some(id) = &query.carrier {
            if let Some(mut bound) = ctx.match_carriers.get(id).cloned() {
                if ctx.capabilities.is_some() {
                    if let Some(arg) =
                        unknown_recall_form(&bound.subs, &query.args, query.plural_sub.as_deref())
                    {
                        return Err(Error::runtime(
                            format!(
                                "PREFLIGHT_UNKNOWN_FORM: unknown form or class `{arg}` on bound `{id}`"
                            ),
                            Some(query.span),
                        ));
                    }
                }
                let (form, _) = form_index(
                    &bound.subs,
                    &query.args,
                    query.plural_sub.as_deref(),
                    ctx.last_number,
                );
                bound.form_index = form;
                let text = bound.printed().to_string();
                return Ok(QueryResult {
                    text,
                    entry: Some(bound),
                });
            }
            if query.table.is_empty() {
                record_unresolved(ctx, query, "unbound");
                return Ok(QueryResult::text(String::new()));
            }
        }
    }

    if query.table.is_empty() {
        return Ok(if query.carrier.is_some() {
            record_unresolved(ctx, query, "unbound");
            QueryResult::text(String::new())
        } else {
            record_unresolved(ctx, query, "unresolved");
            QueryResult::text(format!("<{}>", query.raw))
        });
    }

    let dict = Arc::clone(&ctx.dictionary);
    let Some(table) = dict.table(&query.table) else {
        record_unresolved(ctx, query, "unresolved");
        return Ok(QueryResult::text(format!("<{}>", query.raw)));
    };

    if ctx.capabilities.is_some() {
        if let Some(arg) = unknown_query_form(table, &query.args, query.plural_sub.as_deref()) {
            return Err(Error::runtime(
                format!(
                    "PREFLIGHT_UNKNOWN_FORM: unknown form or class `{arg}` on table `{}`",
                    table.name
                ),
                Some(query.span),
            ));
        }
    }

    let (form, classes) = form_index(
        &table.subs,
        &query.args,
        query.plural_sub.as_deref(),
        ctx.last_number,
    );
    let mut idxs = select_indices(table, &classes, &query.exclude, ctx.nsfw);

    if let Some(pat) = &query.regex {
        idxs = apply_regex(table, idxs, form, pat, query.regex_neg, query.span)?;
    }

    if query.carrier_kind == Some(CarrierKind::Unique) {
        if let Some(id) = &query.carrier {
            if let Some(used) = ctx.unique_carriers.get(id) {
                idxs.retain(|&i| !used.contains(form_of(&table.entries[i], form)));
            }
        }
    }

    if query.carrier_kind == Some(CarrierKind::Rhyme) {
        if let Some(id) = &query.carrier {
            idxs = apply_rhyme(table, idxs, form, id, ctx, query.span)?;
        }
    }

    if idxs.is_empty() {
        if query.carrier_kind == Some(CarrierKind::Rhyme) {
            if let Some(id) = &query.carrier {
                let note = ctx.rhyme_carriers.get(id).map(|group| {
                    format!(
                        "rhyme group `{id}` has no partner for \"{}\" ({})",
                        group.seed_word,
                        ctx.rhyme_mode.as_str()
                    )
                });
                if let Some(note) = note {
                    ctx.notes.push(note);
                }
            }
        }
        record_unresolved(ctx, query, "unresolved");
        return Ok(QueryResult::text(format!("<{}>", query.raw)));
    }

    let pick = *ctx.rng.pick(&idxs).expect("non-empty");
    let entry = &table.entries[pick];
    let text = form_of(entry, form).to_string();
    let bound = BoundEntry::from_table(table, entry, form);

    if let Some(id) = &query.carrier {
        match query.carrier_kind {
            Some(CarrierKind::Unique) => {
                ctx.unique_carriers
                    .entry(id.clone())
                    .or_default()
                    .insert(text.clone());
            }
            Some(CarrierKind::Rhyme) => {
                let phones = phones_for(entry, form, ctx);
                let ident = entry_ident(table, entry);
                let seed_word = text.clone();
                ctx.rhyme_carriers
                    .entry(id.clone())
                    .or_insert_with(|| RhymeGroup {
                        phones,
                        used: Default::default(),
                        seed_word,
                    })
                    .used
                    .insert(ident);
            }
            Some(CarrierKind::Match) | None => {
                ctx.match_carriers.insert(id.clone(), bound.clone());
            }
        }
    }

    record_pick(ctx, query, &bound, &text);

    Ok(QueryResult {
        text,
        entry: Some(bound),
    })
}

fn unknown_recall_form(
    subs: &[String],
    args: &[String],
    plural_sub: Option<&str>,
) -> Option<String> {
    for arg in args {
        if arg != "nsfw" && !subs.iter().any(|s| s == arg) {
            return Some(arg.clone());
        }
    }
    if let Some(pl) = plural_sub {
        if !subs.iter().any(|s| s == pl) {
            return Some(pl.to_string());
        }
    }
    None
}

fn unknown_query_form(
    table: &crate::dict::Table,
    args: &[String],
    plural_sub: Option<&str>,
) -> Option<String> {
    for arg in args {
        if arg == "nsfw" || table.subs.iter().any(|s| s == arg) || table.by_class.contains_key(arg)
        {
            continue;
        }
        return Some(arg.clone());
    }
    if let Some(pl) = plural_sub {
        if !table.subs.iter().any(|s| s == pl) {
            return Some(pl.to_string());
        }
    }
    None
}

fn record_unresolved(ctx: &mut Context, query: &QueryNode, kind: &str) {
    ctx.unresolved.push(UnresolvedQuery {
        kind: kind.to_string(),
        raw: query.raw.clone(),
        table: query.table.clone(),
        carrier: query.carrier.clone(),
        span: query.span,
    });
    if kind == "unbound" {
        ctx.notes.push(format!(
            "unbound carrier '{}'",
            query.carrier.as_deref().unwrap_or("")
        ));
    } else {
        ctx.notes.push(format!("unresolved query <{}>", query.raw));
    }
}

fn record_pick(ctx: &mut Context, query: &QueryNode, bound: &BoundEntry, text: &str) {
    let Some(picks) = ctx.picks.as_mut() else {
        return;
    };
    let (emitted, channel) = match ctx.capture_frames.last() {
        Some(frame) => (frame.will_emit, frame.channel.clone()),
        None => (true, Some(ctx.channel.clone())),
    };
    picks.push(QueryPick {
        table: bound.table.clone(),
        value: text.to_string(),
        entry_id: bound.id.clone(),
        forms: bound.forms.clone(),
        classes: bound.classes.clone(),
        form_index: bound.form_index,
        args: query.args.clone(),
        carrier: query.carrier.clone(),
        span: query.span,
        channel,
        emitted,
    });
}
