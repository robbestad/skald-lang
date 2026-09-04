use super::types::{Dictionary, Entry, Table};
use crate::error::Error;
use crate::span::Span;
use std::collections::HashMap;

pub fn to_json(dict: &Dictionary) -> String {
    let mut out = String::from("{\"tables\":{");
    let mut names: Vec<&String> = dict.tables.keys().collect();
    names.sort();
    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let table = &dict.tables[*name];
        write_str(&mut out, name);
        out.push(':');
        write_table(&mut out, table);
    }
    out.push_str("}}");
    out
}

pub fn from_json(s: &str) -> Result<Dictionary, Error> {
    let mut p = Parser { s, i: 0 };
    p.skip_ws();
    let value = p.parse_value()?;
    p.skip_ws();
    if p.i != p.s.len() {
        return Err(p.err("trailing data after JSON dictionary"));
    }
    dictionary_from_value(value)
}

fn write_table(out: &mut String, table: &Table) {
    out.push('{');
    write_key(out, "name");
    write_str(out, &table.name);
    out.push(',');
    write_key(out, "subs");
    write_str_array(out, &table.subs);
    out.push(',');
    write_key(out, "entries");
    out.push('[');
    for (i, entry) in table.entries.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('{');
        write_key(out, "forms");
        write_str_array(out, &entry.forms);
        out.push(',');
        write_key(out, "classes");
        write_str_array(out, &entry.classes);
        if entry.phones.iter().any(|p| !p.is_empty()) {
            out.push(',');
            write_key(out, "phones");
            write_str_array(out, &entry.phones);
        }
        out.push('}');
    }
    out.push(']');
    out.push('}');
}

fn write_key(out: &mut String, key: &str) {
    write_str(out, key);
    out.push(':');
}

fn write_str_array(out: &mut String, items: &[String]) {
    out.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_str(out, item);
    }
    out.push(']');
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

enum Value {
    Null,
    Bool,
    Str(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    fn object(self, p: &Parser) -> Result<Vec<(String, Value)>, Error> {
        match self {
            Value::Object(o) => Ok(o),
            _ => Err(p.err("expected object")),
        }
    }

    fn field<'a>(obj: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
        obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

fn dictionary_from_value(value: Value) -> Result<Dictionary, Error> {
    let dummy = Parser { s: "", i: 0 };
    let obj = value.object(&dummy)?;
    let mut tables = HashMap::new();
    let mut found = false;
    for (key, val) in obj {
        if key != "tables" {
            continue;
        }
        found = true;
        let tables_obj = val.object(&dummy)?;
        for (name, tval) in tables_obj {
            let table = table_from_value(name.clone(), tval)?;
            tables.insert(name, table);
        }
    }
    if !found {
        return Err(dummy.err("missing tables"));
    }
    let mut dict = Dictionary { tables };
    dict.index();
    Ok(dict)
}

fn table_from_value(fallback: String, value: Value) -> Result<Table, Error> {
    let dummy = Parser { s: "", i: 0 };
    let obj = value.object(&dummy)?;
    let name = match Value::field(&obj, "name") {
        Some(Value::Str(s)) => s.clone(),
        _ => fallback,
    };
    let subs = match Value::field(&obj, "subs") {
        Some(Value::Array(a)) => string_array(a)?,
        _ => vec!["default".to_string()],
    };
    let entries = match Value::field(&obj, "entries") {
        Some(Value::Array(a)) => a
            .iter()
            .map(entry_from_value)
            .collect::<Result<Vec<_>, _>>()?,
        _ => Vec::new(),
    };
    Ok(Table {
        name,
        subs,
        entries,
        by_class: HashMap::new(),
        has_nsfw: false,
    })
}

fn entry_from_value(value: &Value) -> Result<Entry, Error> {
    let dummy = Parser { s: "", i: 0 };
    let obj = match value {
        Value::Object(o) => o,
        _ => return Err(dummy.err("entry must be an object")),
    };
    let forms = match Value::field(obj, "forms") {
        Some(Value::Array(a)) => string_array(a)?,
        _ => Vec::new(),
    };
    let classes = match Value::field(obj, "classes") {
        Some(Value::Array(a)) => string_array(a)?,
        _ => Vec::new(),
    };
    let phones = match Value::field(obj, "phones") {
        Some(Value::Array(a)) => string_array(a)?,
        _ => Vec::new(),
    };
    Ok(Entry {
        forms,
        classes,
        phones,
    })
}

fn string_array(values: &[Value]) -> Result<Vec<String>, Error> {
    let dummy = Parser { s: "", i: 0 };
    values
        .iter()
        .map(|v| match v {
            Value::Str(s) => Ok(s.clone()),
            _ => Err(dummy.err("expected string in array")),
        })
        .collect()
}

struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl Parser<'_> {
    fn err(&self, message: &str) -> Error {
        Error::parse(message, Span::new(self.i, self.i))
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.i += c.len_utf8();
        Some(c)
    }

    fn expect(&mut self, want: char) -> Result<(), Error> {
        self.skip_ws();
        match self.bump() {
            Some(c) if c == want => Ok(()),
            Some(c) => Err(self.err(&format!("expected {want:?}, got {c:?}"))),
            None => Err(self.err(&format!("expected {want:?}, got end"))),
        }
    }

    fn parse_value(&mut self) -> Result<Value, Error> {
        self.skip_ws();
        match self.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => Ok(Value::Str(self.parse_string()?)),
            Some('t') => self.parse_lit("true", Value::Bool),
            Some('f') => self.parse_lit("false", Value::Bool),
            Some('n') => self.parse_lit("null", Value::Null),
            Some('-') | Some('0'..='9') => {
                self.skip_number();
                Ok(Value::Null)
            }
            Some(c) => Err(self.err(&format!("unexpected {c:?}"))),
            None => Err(self.err("unexpected end")),
        }
    }

    fn parse_lit(&mut self, lit: &str, value: Value) -> Result<Value, Error> {
        if self.s[self.i..].starts_with(lit) {
            self.i += lit.len();
            Ok(value)
        } else {
            Err(self.err(&format!("expected {lit}")))
        }
    }

    fn skip_number(&mut self) {
        if self.peek() == Some('-') {
            self.bump();
        }
        while matches!(
            self.peek(),
            Some('0'..='9') | Some('.') | Some('e') | Some('E') | Some('+') | Some('-')
        ) {
            self.bump();
        }
    }

    fn parse_object(&mut self) -> Result<Value, Error> {
        self.expect('{')?;
        let mut fields = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() == Some('}') {
                self.bump();
                break;
            }
            if !fields.is_empty() {
                self.expect(',')?;
                self.skip_ws();
                if self.peek() == Some('}') {
                    self.bump();
                    break;
                }
            }
            let key = self.parse_string()?;
            self.expect(':')?;
            let val = self.parse_value()?;
            fields.push((key, val));
        }
        Ok(Value::Object(fields))
    }

    fn parse_array(&mut self) -> Result<Value, Error> {
        self.expect('[')?;
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() == Some(']') {
                self.bump();
                break;
            }
            if !items.is_empty() {
                self.expect(',')?;
                self.skip_ws();
                if self.peek() == Some(']') {
                    self.bump();
                    break;
                }
            }
            items.push(self.parse_value()?);
        }
        Ok(Value::Array(items))
    }

    fn parse_string(&mut self) -> Result<String, Error> {
        self.skip_ws();
        if self.bump() != Some('"') {
            return Err(self.err("expected string"));
        }
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(self.err("unterminated string")),
                Some('"') => return Ok(out),
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('b') => out.push('\u{0008}'),
                    Some('f') => out.push('\u{000c}'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => {
                        let mut hex = String::new();
                        for _ in 0..4 {
                            match self.bump() {
                                Some(c) => hex.push(c),
                                None => return Err(self.err("bad \\u escape")),
                            }
                        }
                        let code = u32::from_str_radix(&hex, 16)
                            .map_err(|_| self.err("bad \\u escape"))?;
                        out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                    }
                    Some(c) => return Err(self.err(&format!("bad escape {c:?}"))),
                    None => return Err(self.err("unterminated escape")),
                },
                Some(c) => out.push(c),
            }
        }
    }
}
