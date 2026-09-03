use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMode {
    None,
    Default,
    First,
    Word,
    Title,
    Upper,
    Lower,
    Sentence,
}

impl CaseMode {
    pub fn parse(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "none" => Self::None,
            "default" => Self::Default,
            "first" => Self::First,
            "word" => Self::Word,
            "title" => Self::Title,
            "upper" => Self::Upper,
            "lower" => Self::Lower,
            "sentence" => Self::Sentence,
            _ => Self::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierKind {
    Match,
    Unique,
    Rhyme,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Text(TextNode),
    Query(QueryNode),
    Tag(TagNode),
    Block(BlockNode),
    Escape(EscapeNode),
}

impl Node {
    pub fn span(&self) -> Span {
        match self {
            Self::Text(n) => n.span,
            Self::Query(n) => n.span,
            Self::Tag(n) => n.span,
            Self::Block(n) => n.span,
            Self::Escape(n) => n.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextNode {
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryNode {
    pub table: String,
    pub args: Vec<String>,
    pub exclude: Vec<String>,
    pub carrier: Option<String>,
    pub carrier_kind: Option<CarrierKind>,
    pub regex: Option<String>,
    pub regex_neg: bool,
    pub plural_sub: Option<String>,
    pub raw: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TagNode {
    pub name: String,
    pub arg: String,
    pub args: Vec<Vec<Node>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockAlt {
    pub weight: Option<Vec<Node>>,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockNode {
    pub alternatives: Vec<BlockAlt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EscapeNode {
    pub code: String,
    pub span: Span,
}
