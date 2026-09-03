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
            .filter(|p| p.source == PartSource::Dictionary)
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
    pub parts: Vec<OutputPart>,
    pub density: Option<Density>,
    /// Runtime hints (rhyme miss, …). Always filled; usually empty.
    pub notes: Vec<String>,
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
    out.push_str("{\"start\":");
    out.push_str(&pick.span.start.to_string());
    out.push_str(",\"end\":");
    out.push_str(&pick.span.end.to_string());
    out.push_str("}}");
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
