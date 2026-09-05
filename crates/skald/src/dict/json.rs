use super::pack::{
    Capabilities, LANGUAGE_PACK_FORMAT_VERSION, LanguagePack, PackSource, is_known_locale,
};
use super::types::{Dictionary, Entry, Table};
use crate::error::Error;
use crate::span::Span;
use std::collections::{HashMap, HashSet};

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

#[derive(Clone)]
enum Value {
    Null,
    Bool(bool),
    Str(String),
    Number(String),
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

const PACK_KEYS: &[&str] = &[
    "formatVersion",
    "id",
    "locale",
    "contentVersion",
    "capabilities",
    "source",
    "forms",
    "tables",
];
const CAPABILITY_KEYS: &[&str] = &["articles", "numbersVerbal", "caseTitle", "rhyme"];
const SOURCE_KEYS: &[&str] = &["name", "license", "url"];
const TABLE_KEYS: &[&str] = &["name", "subs", "entries"];
const ENTRY_KEYS: &[&str] = &["id", "forms", "classes", "phones"];
const ARTICLE_VALUES: &[&str] = &["en-indefinite", "none"];
const NUMBER_VALUES: &[&str] = &["en", "none"];
const CASE_VALUES: &[&str] = &["en", "none"];

pub fn from_language_pack(s: &str) -> Result<LanguagePack, Error> {
    let mut p = Parser { s, i: 0 };
    p.skip_ws();
    let value = p.parse_value()?;
    p.skip_ws();
    if p.i != p.s.len() {
        return Err(p.err("trailing data after language pack"));
    }
    pack_from_value(value)
}

fn pack_from_value(value: Value) -> Result<LanguagePack, Error> {
    let dummy = Parser { s: "", i: 0 };
    let obj = value.object(&dummy)?;
    reject_unknown(&obj, PACK_KEYS, "language pack")?;
    let format_version = match Value::field(&obj, "formatVersion") {
        Some(Value::Number(n)) => n
            .parse::<u32>()
            .map_err(|_| dummy.err("formatVersion must be a positive integer"))?,
        _ => return Err(dummy.err("language pack requires formatVersion")),
    };
    if format_version != LANGUAGE_PACK_FORMAT_VERSION {
        return Err(dummy.err(&format!(
            "unsupported language pack formatVersion {format_version}"
        )));
    }
    let id = required_str(&obj, "id")?;
    let locale = required_str(&obj, "locale")?;
    if !is_known_locale(&locale) {
        return Err(dummy.err(&format!("unknown locale {locale}")));
    }
    let content_version = required_str(&obj, "contentVersion")?;
    let capabilities = match Value::field(&obj, "capabilities") {
        Some(v) => capabilities_from_value(v, &locale)?,
        None => Capabilities::default_for_locale(&locale),
    };
    let source = match Value::field(&obj, "source") {
        Some(v) => Some(source_from_value(v)?),
        None => None,
    };
    let form_reqs = match Value::field(&obj, "forms") {
        Some(Value::Object(o)) => {
            let mut map = HashMap::new();
            for (name, val) in o {
                let Value::Array(a) = val else {
                    return Err(dummy.err("forms values must be arrays of strings"));
                };
                map.insert(name.clone(), string_array(a)?);
            }
            map
        }
        Some(_) => return Err(dummy.err("forms must be an object")),
        None => HashMap::new(),
    };
    let tables_val = Value::field(&obj, "tables")
        .cloned()
        .ok_or_else(|| dummy.err("missing tables"))?;
    let tables_obj = tables_val.object(&dummy)?;
    let mut tables = HashMap::new();
    let mut ids = HashSet::new();
    for (name, tval) in tables_obj {
        let table = pack_table_from_value(name.clone(), tval, &form_reqs, &mut ids)?;
        if tables.contains_key(&table.name) {
            return Err(dummy.err(&format!("duplicate table {}", table.name)));
        }
        tables.insert(table.name.clone(), table);
    }
    let mut dictionary = Dictionary { tables };
    dictionary.index();
    Ok(LanguagePack {
        id,
        locale,
        format_version,
        content_version,
        capabilities,
        source,
        dictionary,
    })
}

fn reject_unknown(obj: &[(String, Value)], allowed: &[&str], what: &str) -> Result<(), Error> {
    let dummy = Parser { s: "", i: 0 };
    let unknown: Vec<&str> = obj
        .iter()
        .map(|(k, _)| k.as_str())
        .filter(|k| !allowed.contains(k))
        .collect();
    if unknown.is_empty() {
        Ok(())
    } else {
        Err(dummy.err(&format!("unknown {what} fields: {}", unknown.join(", "))))
    }
}

fn required_str(obj: &[(String, Value)], key: &str) -> Result<String, Error> {
    let dummy = Parser { s: "", i: 0 };
    match Value::field(obj, key) {
        Some(Value::Str(s)) if !s.is_empty() => Ok(s.clone()),
        _ => Err(dummy.err(&format!("{key} must be a non-empty string"))),
    }
}

fn capabilities_from_value(value: &Value, locale: &str) -> Result<Capabilities, Error> {
    let dummy = Parser { s: "", i: 0 };
    let obj = match value {
        Value::Object(o) => o,
        _ => return Err(dummy.err("capabilities must be an object")),
    };
    reject_unknown(obj, CAPABILITY_KEYS, "capabilities")?;
    let defaults = Capabilities::default_for_locale(locale);
    let articles = enum_str(obj, "articles", ARTICLE_VALUES, &defaults.articles)?;
    let numbers_verbal = enum_str(
        obj,
        "numbersVerbal",
        NUMBER_VALUES,
        &defaults.numbers_verbal,
    )?;
    let case_title = enum_str(obj, "caseTitle", CASE_VALUES, &defaults.case_title)?;
    let rhyme = match Value::field(obj, "rhyme") {
        Some(Value::Bool(v)) => *v,
        None => defaults.rhyme,
        _ => return Err(dummy.err("rhyme must be a boolean")),
    };
    Ok(Capabilities {
        articles,
        numbers_verbal,
        case_title,
        rhyme,
    })
}

fn enum_str(
    obj: &[(String, Value)],
    key: &str,
    allowed: &[&str],
    default: &str,
) -> Result<String, Error> {
    let dummy = Parser { s: "", i: 0 };
    match Value::field(obj, key) {
        None => Ok(default.to_string()),
        Some(Value::Str(s)) if allowed.contains(&s.as_str()) => Ok(s.clone()),
        Some(Value::Str(_)) => {
            Err(dummy.err(&format!("{key} must be one of {}", allowed.join(", "))))
        }
        _ => Err(dummy.err(&format!("{key} must be a string"))),
    }
}

fn source_from_value(value: &Value) -> Result<PackSource, Error> {
    let dummy = Parser { s: "", i: 0 };
    let obj = match value {
        Value::Object(o) => o,
        _ => return Err(dummy.err("source must be an object")),
    };
    reject_unknown(obj, SOURCE_KEYS, "source")?;
    Ok(PackSource {
        name: required_str(obj, "name")?,
        license: required_str(obj, "license")?,
        url: match Value::field(obj, "url") {
            Some(Value::Str(s)) if !s.is_empty() => Some(s.clone()),
            Some(_) => return Err(dummy.err("source.url must be a non-empty string")),
            None => None,
        },
    })
}

fn pack_table_from_value(
    fallback: String,
    value: Value,
    form_reqs: &HashMap<String, Vec<String>>,
    ids: &mut HashSet<String>,
) -> Result<Table, Error> {
    let dummy = Parser { s: "", i: 0 };
    let obj = value.object(&dummy)?;
    reject_unknown(&obj, TABLE_KEYS, "table")?;
    let name = match Value::field(&obj, "name") {
        Some(Value::Str(s)) => s.clone(),
        None => fallback,
        _ => return Err(dummy.err("table name must be a string")),
    };
    let mut subs = match Value::field(&obj, "subs") {
        Some(Value::Array(a)) => string_array(a)?,
        None => Vec::new(),
        _ => return Err(dummy.err("subs must be an array of strings")),
    };
    let entries = match Value::field(&obj, "entries") {
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| pack_entry_from_value(v, ids))
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
        _ => return Err(dummy.err("entries must be an array")),
    };
    if let Some(required) = form_reqs.get(&name) {
        if subs.is_empty() {
            subs = required.clone();
        } else if subs != *required {
            return Err(dummy.err(&format!("table {name} subs must match forms.{name}")));
        }
    }
    if subs.is_empty() {
        subs = vec!["default".to_string()];
    }
    let want = subs.len();
    for entry in &entries {
        if entry.forms.len() != want {
            return Err(dummy.err(&format!(
                "table {name} entries must have {want} forms; missing forms are not filled from the lemma"
            )));
        }
    }
    Ok(Table {
        name,
        subs,
        entries,
        by_class: HashMap::new(),
        has_nsfw: false,
    })
}

fn pack_entry_from_value(value: &Value, ids: &mut HashSet<String>) -> Result<Entry, Error> {
    let dummy = Parser { s: "", i: 0 };
    let obj = match value {
        Value::Object(o) => o,
        _ => return Err(dummy.err("entry must be an object")),
    };
    reject_unknown(obj, ENTRY_KEYS, "entry")?;
    let id = match Value::field(obj, "id") {
        Some(Value::Str(s)) if !s.is_empty() => {
            if !ids.insert(s.clone()) {
                return Err(dummy.err(&format!("duplicate entry id {s}")));
            }
            Some(s.clone())
        }
        Some(_) => return Err(dummy.err("entry id must be a non-empty string")),
        None => {
            return Err(dummy.err(
                "language pack entries require a stable id; omit formatVersion to load a legacy dictionary",
            ))
        }
    };
    let forms = match Value::field(obj, "forms") {
        Some(Value::Array(a)) => string_array(a)?,
        _ => return Err(dummy.err("entry forms must be an array of strings")),
    };
    if forms.is_empty() {
        return Err(dummy.err("entry forms must not be empty"));
    }
    let classes = match Value::field(obj, "classes") {
        Some(Value::Array(a)) => string_array(a)?,
        None => Vec::new(),
        _ => return Err(dummy.err("classes must be an array of strings")),
    };
    let phones = match Value::field(obj, "phones") {
        Some(Value::Array(a)) => string_array(a)?,
        None => Vec::new(),
        _ => return Err(dummy.err("phones must be an array of strings")),
    };
    Ok(Entry {
        id,
        forms,
        classes,
        phones,
    })
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
        id: match Value::field(obj, "id") {
            Some(Value::Str(s)) if !s.is_empty() => Some(s.clone()),
            _ => None,
        },
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
            Some('t') => self.parse_lit("true", Value::Bool(true)),
            Some('f') => self.parse_lit("false", Value::Bool(false)),
            Some('n') => self.parse_lit("null", Value::Null),
            Some('-') | Some('0'..='9') => Ok(Value::Number(self.parse_number_text())),
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

    fn parse_number_text(&mut self) -> String {
        let start = self.i;
        if self.peek() == Some('-') {
            self.bump();
        }
        while matches!(
            self.peek(),
            Some('0'..='9') | Some('.') | Some('e') | Some('E') | Some('+') | Some('-')
        ) {
            self.bump();
        }
        self.s[start..self.i].to_string()
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
