use crate::ast::Node;
use crate::dict::BoundEntry;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    #[default]
    Nil,
    Bool(bool),
    Int(i64),
    Str(String),
    List(Vec<Value>),
    Entry(BoundEntry),
    /// Unevaluated pattern fragment. Printed by running it.
    Pattern(Vec<Node>),
    /// Named bag. Printed empty; read with `[name: key]`.
    Map(Vec<(String, Value)>),
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
            Self::Pattern(_) | Self::Map(_) => String::new(),
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
            Self::Pattern(nodes) => nodes.len() as i64,
            Self::Map(pairs) => pairs.len() as i64,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }
}
