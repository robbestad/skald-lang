use super::index::build_table_index;
use super::types::{Dictionary, Entry, Table};
use crate::aliases::resolve_table_name;

pub fn compile_dic(source: &str, fallback_name: &str) -> Table {
    let mut name = fallback_name.to_string();
    let mut subs = vec!["default".to_string()];
    let mut class_stack: Vec<String> = Vec::new();
    let mut entries: Vec<Entry> = Vec::new();
    let mut last: Option<usize> = None;

    for raw in source.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(body) = line.strip_prefix('#') {
            let body = body.trim();
            let mut parts = body.split_whitespace();
            let cmd = parts.next().unwrap_or("").to_ascii_lowercase();
            match cmd.as_str() {
                "version" => {}
                "name" => {
                    let arg = body
                        .split_once(char::is_whitespace)
                        .map(|(_, rest)| rest.trim())
                        .unwrap_or("");
                    name = if arg.is_empty() {
                        fallback_name.to_string()
                    } else {
                        arg.to_string()
                    };
                }
                "subs" => {
                    let rest: Vec<String> = parts.map(|s| s.to_string()).collect();
                    subs = if rest.is_empty() {
                        vec!["default".to_string()]
                    } else {
                        rest
                    };
                }
                "class" => {
                    let op = parts.next().unwrap_or("").to_ascii_lowercase();
                    let class_name = parts.collect::<Vec<_>>().join(" ");
                    if class_name.is_empty() {
                        continue;
                    }
                    if op == "add" {
                        class_stack.push(class_name);
                    } else if op == "remove" {
                        if let Some(idx) = class_stack.iter().rposition(|c| c == &class_name) {
                            class_stack.remove(idx);
                        }
                    }
                }
                "nsfw" => class_stack.push("nsfw".to_string()),
                "sfw" => {
                    if let Some(idx) = class_stack.iter().rposition(|c| c == "nsfw") {
                        class_stack.remove(idx);
                    }
                }
                _ => {}
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix('>') {
            let word = rest.strip_prefix(' ').unwrap_or(rest).trim();
            if word.is_empty() || word == "|" {
                continue;
            }
            let forms: Vec<String> = word.split('/').map(|f| f.trim().to_string()).collect();
            let classes = active_classes(&class_stack);
            entries.push(Entry {
                forms,
                classes,
                phones: Vec::new(),
            });
            last = Some(entries.len() - 1);
            continue;
        }

        if let Some(rest) = line.strip_prefix('|') {
            let Some(idx) = last else { continue };
            let meta = rest.strip_prefix(' ').unwrap_or(rest);
            if let Some(rest) = meta_cmd(meta, "pron") {
                entries[idx].phones = rest.split('/').map(|s| s.trim().to_string()).collect();
                continue;
            }
            if meta_cmd(meta, "weight").is_some() {
                continue;
            }
            let lower = meta.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("class ") {
                let extra: Vec<String> = rest
                    .split_whitespace()
                    .map(|s| s.trim().to_ascii_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                let entry = &mut entries[idx];
                for c in extra {
                    if !entry.classes.contains(&c) {
                        entry.classes.push(c);
                    }
                }
            }
        }
    }

    let resolved = resolve_table_name(&name);
    let (by_class, has_nsfw) = build_table_index(&entries);
    Table {
        name: resolved,
        subs: subs.into_iter().map(|s| s.to_ascii_lowercase()).collect(),
        entries,
        by_class,
        has_nsfw,
    }
}

fn meta_cmd<'a>(meta: &'a str, cmd: &str) -> Option<&'a str> {
    let meta = meta.trim();
    if meta.len() < cmd.len() || !meta[..cmd.len()].eq_ignore_ascii_case(cmd) {
        return None;
    }
    let rest = &meta[cmd.len()..];
    if rest.is_empty() {
        return Some("");
    }
    let first = rest.chars().next()?;
    if first.is_whitespace() || first == ':' {
        Some(rest.trim_start_matches(|c: char| c == ':' || c.is_whitespace()))
    } else {
        None
    }
}

fn active_classes(stack: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for c in stack {
        let n = c.to_ascii_lowercase();
        if seen.insert(n.clone()) {
            out.push(n);
        }
    }
    out
}

pub fn compile_dictionaries(files: &[(&str, &str)]) -> Dictionary {
    let mut tables: std::collections::HashMap<String, Table> = std::collections::HashMap::new();
    for (name, source) in files {
        let fallback = name
            .rsplit('/')
            .next()
            .unwrap_or(name)
            .trim_end_matches(".dic")
            .trim_end_matches(".DIC")
            .replace(' ', "_")
            .to_ascii_lowercase();
        let mut table = compile_dic(source, &fallback);
        let aliased = resolve_table_name(&table.name);
        table.name = aliased.clone();
        if let Some(existing) = tables.get_mut(&aliased) {
            existing.entries.extend(table.entries);
            if existing.subs.len() < table.subs.len() {
                existing.subs = table.subs;
            }
        } else {
            tables.insert(aliased, table);
        }
    }
    let mut dict = Dictionary { tables };
    dict.index();
    dict
}
