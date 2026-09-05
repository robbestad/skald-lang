use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Stable id inside a language pack. Legacy dictionaries may omit it.
    pub id: Option<String>,
    pub forms: Vec<String>,
    pub classes: Vec<String>,
    /// Rantionary SAMPA for each form. Empty string = missing for that form.
    pub phones: Vec<String>,
}

/// A match-carrier binding: the whole dictionary row, not just the printed form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundEntry {
    pub table: String,
    pub id: Option<String>,
    pub forms: Vec<String>,
    pub classes: Vec<String>,
    pub phones: Vec<String>,
    pub subs: Vec<String>,
    pub form_index: usize,
}

impl BoundEntry {
    pub fn from_table(table: &Table, entry: &Entry, form_index: usize) -> Self {
        Self {
            table: table.name.clone(),
            id: entry.id.clone(),
            forms: entry.forms.clone(),
            classes: entry.classes.clone(),
            phones: entry.phones.clone(),
            subs: table.subs.clone(),
            form_index,
        }
    }

    pub fn phones(&self, index: usize) -> &str {
        self.phones.get(index).map(String::as_str).unwrap_or("")
    }

    pub fn form(&self, index: usize) -> &str {
        self.forms
            .get(index)
            .or_else(|| self.forms.first())
            .map(String::as_str)
            .unwrap_or("")
    }

    pub fn printed(&self) -> &str {
        self.form(self.form_index)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub name: String,
    pub subs: Vec<String>,
    pub entries: Vec<Entry>,
    pub by_class: HashMap<String, Vec<usize>>,
    pub has_nsfw: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dictionary {
    pub tables: HashMap<String, Table>,
}

impl Dictionary {
    pub fn empty() -> Self {
        Self {
            tables: HashMap::new(),
        }
    }

    pub fn index(&mut self) {
        for table in self.tables.values_mut() {
            let (by_class, has_nsfw) = super::index::build_table_index(&table.entries);
            table.by_class = by_class;
            table.has_nsfw = has_nsfw;
        }
    }

    pub fn table(&self, name: &str) -> Option<&Table> {
        let mapped = crate::aliases::resolve_table_name(name);
        self.tables.get(&mapped).or_else(|| self.tables.get(name))
    }

    /// Insert or replace tables from `extra`. Same names are replaced, not appended.
    pub fn overlay(&mut self, extra: &Dictionary) {
        for (name, table) in &extra.tables {
            self.tables.insert(name.clone(), table.clone());
        }
        self.index();
    }
}
