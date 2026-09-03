use crate::ast::{CaseMode, Node};
use crate::dict::{BoundEntry, Dictionary};
use crate::error::Error;
use crate::output::QueryPick;
use crate::rhyme::{RhymeGroup, RhymeMode};
use crate::rng::Rng;
use crate::span::Span;
use crate::sync::{SyncState, SyncType, next_index};
use crate::value::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub const MAX_STEPS: u32 = 100_000;
pub const MAX_OUTPUT: usize = 1_000_000;
pub const MAX_DEPTH: u32 = 64;

#[derive(Debug, Clone)]
pub struct UserFn {
    pub params: Vec<String>,
    pub body: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct FnPending {
    pub name: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BlockAttrs {
    pub rep: Rep,
    pub sep: Option<Vec<Node>>,
    pub sync: Option<String>,
    pub chance: f64,
    pub fn_def: Option<FnPending>,
    pub out: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Rep {
    Times(i64),
    Each,
}

impl Default for BlockAttrs {
    fn default() -> Self {
        Self {
            rep: Rep::Times(1),
            sep: None,
            sync: None,
            chance: 100.0,
            fn_def: None,
            out: None,
        }
    }
}

pub struct Context {
    pub rng: Rng,
    pub dictionary: Arc<Dictionary>,
    pub nsfw: bool,
    pub match_carriers: HashMap<String, BoundEntry>,
    pub unique_carriers: HashMap<String, HashSet<String>>,
    pub rhyme_carriers: HashMap<String, RhymeGroup>,
    pub rhyme_mode: RhymeMode,
    pub case_mode: CaseMode,
    pub numfmt: String,
    pub channels: HashMap<String, String>,
    pub channel: String,
    pub capture: Vec<String>,
    pub attrs: BlockAttrs,
    pub syncs: HashMap<String, SyncState>,
    pub pending_article: bool,
    pub pending_if: Option<String>,
    pub bindings: Vec<HashMap<String, Value>>,
    pub functions: HashMap<String, UserFn>,
    pub call_depth: u32,
    pub rep_index: i64,
    pub steps: u32,
    pub last_number: Option<i64>,
    pub picks: Option<Vec<QueryPick>>,
}

impl Context {
    pub fn new(rng: Rng, case_mode: CaseMode, dictionary: Arc<Dictionary>) -> Self {
        Self {
            rng,
            dictionary,
            nsfw: false,
            match_carriers: HashMap::new(),
            unique_carriers: HashMap::new(),
            rhyme_carriers: HashMap::new(),
            rhyme_mode: RhymeMode::Perfect,
            case_mode,
            numfmt: "normal".to_string(),
            channels: HashMap::from([("main".to_string(), String::new())]),
            channel: "main".to_string(),
            capture: Vec::new(),
            attrs: BlockAttrs::default(),
            syncs: HashMap::new(),
            pending_article: false,
            pending_if: None,
            bindings: vec![HashMap::new()],
            functions: HashMap::new(),
            call_depth: 0,
            rep_index: 0,
            steps: 0,
            last_number: None,
            picks: None,
        }
    }

    pub fn tick(&mut self, span: Span) -> Result<(), Error> {
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return Err(Error::budget(format!(
                "exceeded {MAX_STEPS} steps (at {}..{})",
                span.start, span.end
            )));
        }
        Ok(())
    }

    pub fn write(&mut self, s: &str) -> Result<(), Error> {
        if s.is_empty() {
            return Ok(());
        }
        let next = self.output_len() + s.len();
        if next > MAX_OUTPUT {
            return Err(Error::budget(format!("exceeded {MAX_OUTPUT} output bytes")));
        }
        if let Some(last) = self.capture.last_mut() {
            last.push_str(s);
        } else {
            self.write_channel(&self.channel.clone(), s)?;
        }
        Ok(())
    }

    pub fn write_channel(&mut self, name: &str, s: &str) -> Result<(), Error> {
        if s.is_empty() {
            return Ok(());
        }
        let key = if name.is_empty() { "main" } else { name };
        let next = self.channels.values().map(String::len).sum::<usize>() + s.len();
        if next > MAX_OUTPUT {
            return Err(Error::budget(format!("exceeded {MAX_OUTPUT} output bytes")));
        }
        self.channels
            .entry(key.to_string())
            .or_default()
            .push_str(s);
        Ok(())
    }

    fn output_len(&self) -> usize {
        if let Some(last) = self.capture.last() {
            last.len()
        } else {
            self.channels.values().map(String::len).sum()
        }
    }

    pub fn lookup_binding(&self, name: &str) -> Option<&Value> {
        self.bindings.iter().rev().find_map(|frame| frame.get(name))
    }

    pub fn bind(&mut self, name: String, value: Value) {
        if let Some(frame) = self.bindings.last_mut() {
            frame.insert(name, value);
        }
    }

    pub fn push_frame(&mut self) {
        self.bindings.push(HashMap::new());
    }

    pub fn pop_frame(&mut self) {
        if self.bindings.len() > 1 {
            self.bindings.pop();
        }
    }

    pub fn pick_synced(&mut self, name: &str, count: usize) -> usize {
        let state = self
            .syncs
            .entry(name.to_string())
            .or_insert_with(|| SyncState::new(SyncType::None));
        next_index(state, count, &mut self.rng)
    }
}

pub fn capture<F>(ctx: &mut Context, f: F) -> Result<String, Error>
where
    F: FnOnce(&mut Context) -> Result<(), Error>,
{
    ctx.capture.push(String::new());
    f(ctx)?;
    Ok(ctx.capture.pop().unwrap_or_default())
}
