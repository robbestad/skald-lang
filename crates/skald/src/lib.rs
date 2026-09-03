//! Skald — a dictionary-native generative language.
//!
//! Queries return dictionary entries. Match carriers bind the row, so
//! `<::hero plural>` is the same person in another form.

mod aliases;
mod ast;
mod dict;
mod engine;
mod error;
mod format;
mod functions;
mod interpret;
mod output;
mod parse;
mod query;
mod rhyme;
mod rng;
mod runtime;
mod span;
mod sync;
mod value;

pub use ast::{
    BlockAlt, BlockNode, CarrierKind, CaseMode, EscapeNode, Node, QueryNode, TagNode, TextNode,
};
pub use dict::{
    BoundEntry, Dictionary, Entry, Table, compile_dic, compile_dictionaries, en_us, from_json,
    to_json,
};
pub use engine::{Options, Program, compile, explain, skald, skald_output};
pub use error::Error;
pub use output::{Output, QueryPick};
pub use parse::{Token, TokenKind, parse, tokenize};
pub use rng::{Rng, Seed};
pub use span::Span;
pub use value::Value;
