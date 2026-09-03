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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub text: String,
    pub channels: HashMap<String, String>,
    /// Filled only when the run was an `explain()`.
    pub picks: Vec<QueryPick>,
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
        out.push_str("]}");
        out
    }
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
