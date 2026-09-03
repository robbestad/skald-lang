//! Skald — a dictionary-native generative language.
//!
//! Write a pattern. Get a sentence whose words did not come from a model.
//! Queries return dictionary entries. Match carriers bind the row, so
//! `<::hero plural>` is the same person in another form.
//!
//! # Examples
//!
//! Fill a pattern. Same seed, same sentence.
//!
//! ```
//! use skald::{skald, CaseMode, Options, Seed};
//!
//! let line = skald(
//!     "<firstname male> found [a] <noun-animal>.",
//!     &Options {
//!         seed: Some(Seed::Int(42)),
//!         case_mode: Some(CaseMode::None),
//!         ..Options::default()
//!     },
//! )
//! .unwrap();
//! assert!(!line.contains('<'));
//! ```
//!
//! `explain` / `--prove` splits lexicon rows from glue.
//!
//! ```
//! use skald::{explain, CaseMode, Options, Seed};
//!
//! let out = explain(
//!     "<firstname male> found [a] <noun-animal>.",
//!     &Options {
//!         seed: Some(Seed::Int(42)),
//!         case_mode: Some(CaseMode::None),
//!         ..Options::default()
//!     },
//! )
//! .unwrap();
//! assert!(out.picks.iter().any(|p| p.table == "firstname"));
//! assert!(out.parts.iter().any(|p| p.text.contains("found")));
//! ```
//!
//! Compile once, run many seeds.
//!
//! ```
//! use skald::{compile, CaseMode, Options, Seed};
//!
//! let program = compile("[rep:3]{A|B}").unwrap();
//! let a = program
//!     .run(&Options {
//!         seed: Some(Seed::Int(1)),
//!         case_mode: Some(CaseMode::None),
//!         ..Options::default()
//!     })
//!     .unwrap();
//! let b = program
//!     .run(&Options {
//!         seed: Some(Seed::Int(1)),
//!         case_mode: Some(CaseMode::None),
//!         ..Options::default()
//!     })
//!     .unwrap();
//! assert_eq!(a, b);
//! ```

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
pub use output::{Density, Output, OutputPart, PartSource, QueryPick};
pub use parse::{Token, TokenKind, parse, tokenize};
pub use rhyme::parse_pron_sidecar;
pub use rng::{Rng, Seed};
pub use runtime::Budget;
pub use span::Span;
pub use value::Value;
