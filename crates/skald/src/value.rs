use crate::dict::BoundEntry;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Str(String),
    List(Vec<Value>),
    Entry(BoundEntry),
}

impl Value {
    pub fn to_print(&self) -> String {
        match self {
            Self::Nil => String::new(),
            Self::Bool(true) => "true".to_string(),
            Self::Bool(false) => "false".to_string(),
            Self::Int(n) => n.to_string(),
            Self::Str(s) => s.clone(),
            Self::List(items) => items.iter().map(Self::to_print).collect(),
            Self::Entry(e) => e.printed().to_string(),
        }
    }

    pub fn len(&self) -> i64 {
        match self {
            Self::Nil => 0,
            Self::Bool(_) => 1,
            Self::Int(n) => n.abs().to_string().len() as i64,
            Self::Str(s) => s.chars().count() as i64,
            Self::List(items) => items.len() as i64,
            Self::Entry(e) => e.printed().chars().count() as i64,
        }
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::Nil
    }
}
