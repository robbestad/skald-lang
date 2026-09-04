use crate::span::Span;
use std::collections::HashMap;

/// One dictionary row chosen while running a pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryPick {
    pub table: String,
    pub value: String,
    pub forms: Vec<String>,
    pub classes: Vec<String>,
    pub form_index: usize,
    pub args: Vec<String>,
    pub carrier: Option<String>,
    pub span: Span,
    /// Channel this pick was emitted to, if any.
    pub channel: Option<String>,
    /// False for `[let]` / argument evaluation that never hit a channel.
    pub emitted: bool,
}

/// A query that did not resolve to a dictionary row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedQuery {
    pub kind: String,
    pub raw: String,
    pub table: String,
    pub carrier: Option<String>,
    pub span: Span,
}

/// A non-query decision recorded in the execution trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub kind: String,
    pub span: Span,
    pub alternative: usize,
    pub repeat_index: i64,
    pub channel: Option<String>,
    pub emitted: bool,
}

/// Machine-readable lint or runtime finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub beat_index: Option<usize>,
    pub span: Option<Span>,
    pub message: String,
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            severity: "error".to_string(),
            beat_index: None,
            span: None,
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_beat(mut self, beat_index: usize, span: Span) -> Self {
        self.beat_index = Some(beat_index);
        self.span = Some(span);
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Compatible `story: …` line for notes / CLI.
    pub fn to_note(&self) -> String {
        let beat = self
            .beat_index
            .map(|i| format!("beat {} ", i + 1))
            .unwrap_or_default();
        format!("story: {beat}{}", self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartSource {
    Dictionary,
    Glue,
}

impl PartSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dictionary => "dictionary",
            Self::Glue => "glue",
        }
    }
}

/// Rewrite part texts so their concatenation equals `cased`, keeping origin.
pub fn rewrite_part_texts(parts: &mut [OutputPart], cased: &str) {
    let original: Vec<(usize, char)> = parts
        .iter()
        .enumerate()
        .flat_map(|(part, value)| value.text.chars().map(move |ch| (part, ch)))
        .collect();
    let mut rewritten = vec![String::new(); parts.len()];
    let mut remaining = cased;
    for (part, ch) in original {
        let original = ch.to_string();
        let upper = ch.to_uppercase().collect::<String>();
        let lower = ch.to_lowercase().collect::<String>();
        let chosen = [upper.as_str(), lower.as_str(), original.as_str()]
            .into_iter()
            .filter(|candidate| remaining.starts_with(candidate))
            .max_by_key(|candidate| candidate.len());
        if let Some(candidate) = chosen {
            rewritten[part].push_str(candidate);
            remaining = &remaining[candidate.len()..];
        } else if let Some(next) = remaining.chars().next() {
            rewritten[part].push(next);
            remaining = &remaining[next.len_utf8()..];
        }
    }
    if !remaining.is_empty() {
        if let Some(last) = rewritten.last_mut() {
            last.push_str(remaining);
        }
    }
    for (part, text) in parts.iter_mut().zip(rewritten) {
        part.text = text;
    }
}

/// Slice lineage covering byte range `[start, end)` of concatenated part text.
pub fn slice_parts(parts: &[OutputPart], start: usize, end: usize) -> Vec<OutputPart> {
    if start >= end {
        return Vec::new();
    }
    let mut acc = 0usize;
    let mut out = Vec::new();
    for part in parts {
        let plen = part.text.len();
        let a = acc;
        let b = acc + plen;
        acc = b;
        let lo = start.max(a);
        let hi = end.min(b);
        if lo < hi {
            out.push(OutputPart {
                text: part.text[lo - a..hi - a].to_string(),
                source: part.source,
                table: part.table.clone(),
            });
        }
        if acc >= end {
            break;
        }
    }
    out
}

/// One run of output: dictionary fill or pattern glue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputPart {
    pub text: String,
    pub source: PartSource,
    pub table: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Density {
    /// Glue characters / output characters, 0..=1.
    pub glue_ratio: f64,
    pub queries: usize,
    pub warning: Option<String>,
}

impl Density {
    pub fn from_parts(parts: &[OutputPart]) -> Self {
        let total: usize = parts.iter().map(|p| p.text.chars().count()).sum();
        let glue: usize = parts
            .iter()
            .filter(|p| p.source == PartSource::Glue)
            .map(|p| p.text.chars().count())
            .sum();
        let queries = parts
            .iter()
            .filter(|p| p.source == PartSource::Dictionary && !p.text.is_empty())
            .count();
        let glue_ratio = if total == 0 {
            0.0
        } else {
            glue as f64 / total as f64
        };
        let warning = if glue_ratio >= 0.5 && total >= 8 {
            Some(format!(
                "output is {:.0}% glue — it will still read like the template",
                glue_ratio * 100.0
            ))
        } else {
            None
        };
        Self {
            glue_ratio,
            queries,
            warning,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub text: String,
    pub channels: HashMap<String, String>,
    /// Filled only when the run was an `explain()`.
    pub picks: Vec<QueryPick>,
    /// Alias for `parts_by_channel["main"]` (final-output lineage).
    pub parts: Vec<OutputPart>,
    /// Final-output lineage per channel. `parts` is `parts_by_channel["main"]`.
    pub parts_by_channel: HashMap<String, Vec<OutputPart>>,
    pub density: Option<Density>,
    /// Runtime hints (rhyme miss, …) plus rendered story notes.
    pub notes: Vec<String>,
    /// Block (and later other) decisions. Explain only.
    pub choices: Vec<Choice>,
    /// Structured lint / policy findings.
    pub diagnostics: Vec<Diagnostic>,
    /// Queries that stayed raw or unbound, with original source spans.
    pub unresolved: Vec<UnresolvedQuery>,
}

impl Output {
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\"text\":");
        json_str(&mut out, &self.text);
        out.push_str(",\"channels\":{");
        let mut names: Vec<&String> = self.channels.keys().collect();
        names.sort();
        for (i, name) in names.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            json_str(&mut out, name);
            out.push(':');
            json_str(&mut out, &self.channels[*name]);
        }
        out.push_str("},\"picks\":[");
        for (i, pick) in self.picks.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write_pick(&mut out, pick);
        }
        out.push_str("],\"parts\":[");
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write_part(&mut out, part);
        }
        out.push(']');
        out.push_str(",\"partsByChannel\":{");
        let mut ch_names: Vec<&String> = self.parts_by_channel.keys().collect();
        ch_names.sort();
        for (i, name) in ch_names.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            json_str(&mut out, name);
            out.push_str(":[");
            for (j, part) in self.parts_by_channel[*name].iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                write_part(&mut out, part);
            }
            out.push(']');
        }
        out.push('}');
        if !self.choices.is_empty() {
            out.push_str(",\"choices\":[");
            for (i, choice) in self.choices.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_choice(&mut out, choice);
            }
            out.push(']');
        }
        if !self.diagnostics.is_empty() {
            out.push_str(",\"diagnostics\":[");
            for (i, d) in self.diagnostics.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_diagnostic(&mut out, d);
            }
            out.push(']');
        }
        if !self.unresolved.is_empty() {
            out.push_str(",\"unresolved\":[");
            for (i, u) in self.unresolved.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_unresolved(&mut out, u);
            }
            out.push(']');
        }
        if let Some(d) = &self.density {
            out.push_str(",\"density\":{");
            write_key(&mut out, "glue_ratio");
            out.push_str(&format!("{:.4}", d.glue_ratio));
            out.push(',');
            write_key(&mut out, "queries");
            out.push_str(&d.queries.to_string());
            if let Some(w) = &d.warning {
                out.push(',');
                write_key(&mut out, "warning");
                json_str(&mut out, w);
            }
            out.push('}');
        }
        if !self.notes.is_empty() {
            out.push_str(",\"notes\":");
            json_str_array(&mut out, &self.notes);
        }
        out.push('}');
        out
    }
}

fn write_part(out: &mut String, part: &OutputPart) {
    out.push('{');
    write_key(out, "text");
    json_str(out, &part.text);
    out.push(',');
    write_key(out, "source");
    json_str(out, part.source.as_str());
    if let Some(table) = &part.table {
        out.push(',');
        write_key(out, "table");
        json_str(out, table);
    }
    out.push('}');
}

fn write_pick(out: &mut String, pick: &QueryPick) {
    out.push('{');
    write_key(out, "table");
    json_str(out, &pick.table);
    out.push(',');
    write_key(out, "value");
    json_str(out, &pick.value);
    out.push(',');
    write_key(out, "forms");
    json_str_array(out, &pick.forms);
    out.push(',');
    write_key(out, "classes");
    json_str_array(out, &pick.classes);
    out.push(',');
    write_key(out, "form");
    out.push_str(&pick.form_index.to_string());
    out.push(',');
    write_key(out, "args");
    json_str_array(out, &pick.args);
    if let Some(carrier) = &pick.carrier {
        out.push(',');
        write_key(out, "carrier");
        json_str(out, carrier);
    }
    out.push(',');
    write_key(out, "span");
    write_span(out, pick.span);
    out.push(',');
    write_key(out, "channel");
    match &pick.channel {
        Some(ch) => json_str(out, ch),
        None => out.push_str("null"),
    }
    out.push(',');
    write_key(out, "emitted");
    out.push_str(if pick.emitted { "true" } else { "false" });
    out.push('}');
}

fn write_choice(out: &mut String, choice: &Choice) {
    out.push('{');
    write_key(out, "kind");
    json_str(out, &choice.kind);
    out.push(',');
    write_key(out, "span");
    write_span(out, choice.span);
    out.push(',');
    write_key(out, "alternative");
    out.push_str(&choice.alternative.to_string());
    out.push(',');
    write_key(out, "repeatIndex");
    out.push_str(&choice.repeat_index.to_string());
    out.push(',');
    write_key(out, "channel");
    match &choice.channel {
        Some(ch) => json_str(out, ch),
        None => out.push_str("null"),
    }
    out.push(',');
    write_key(out, "emitted");
    out.push_str(if choice.emitted { "true" } else { "false" });
    out.push('}');
}

fn write_unresolved(out: &mut String, u: &UnresolvedQuery) {
    out.push('{');
    write_key(out, "kind");
    json_str(out, &u.kind);
    out.push(',');
    write_key(out, "raw");
    json_str(out, &u.raw);
    out.push(',');
    write_key(out, "table");
    json_str(out, &u.table);
    if let Some(carrier) = &u.carrier {
        out.push(',');
        write_key(out, "carrier");
        json_str(out, carrier);
    }
    out.push(',');
    write_key(out, "span");
    write_span(out, u.span);
    out.push('}');
}

fn write_diagnostic(out: &mut String, d: &Diagnostic) {
    out.push('{');
    write_key(out, "code");
    json_str(out, &d.code);
    out.push(',');
    write_key(out, "severity");
    json_str(out, &d.severity);
    out.push(',');
    write_key(out, "beatIndex");
    match d.beat_index {
        Some(i) => out.push_str(&i.to_string()),
        None => out.push_str("null"),
    }
    out.push(',');
    write_key(out, "span");
    match d.span {
        Some(span) => write_span(out, span),
        None => out.push_str("null"),
    }
    out.push(',');
    write_key(out, "message");
    json_str(out, &d.message);
    if let Some(hint) = &d.hint {
        out.push(',');
        write_key(out, "hint");
        json_str(out, hint);
    }
    out.push('}');
}

fn write_span(out: &mut String, span: Span) {
    out.push_str("{\"start\":");
    out.push_str(&span.start.to_string());
    out.push_str(",\"end\":");
    out.push_str(&span.end.to_string());
    out.push('}');
}

fn write_key(out: &mut String, key: &str) {
    json_str(out, key);
    out.push(':');
}

fn json_str_array(out: &mut String, items: &[String]) {
    out.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        json_str(out, item);
    }
    out.push(']');
}

fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::{OutputPart, PartSource, rewrite_part_texts};

    #[test]
    fn casing_expansion_preserves_text_and_lineage() {
        let mut parts = vec![
            OutputPart {
                text: "ß".to_string(),
                source: PartSource::Glue,
                table: None,
            },
            OutputPart {
                text: "x".to_string(),
                source: PartSource::Dictionary,
                table: Some("test".to_string()),
            },
        ];
        rewrite_part_texts(&mut parts, "SSX");
        assert_eq!(parts[0].text, "SS");
        assert_eq!(parts[1].text, "X");
        assert_eq!(
            parts.iter().map(|p| p.text.as_str()).collect::<String>(),
            "SSX"
        );
    }
}
