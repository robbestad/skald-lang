use crate::ast::{CaseMode, Node};
use crate::dict::{BoundEntry, Dictionary};
use crate::error::Error;
use crate::format::case::apply_case;
use crate::output::{Choice, OutputPart, PartSource, QueryPick, UnresolvedQuery};
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

/// Caps for one run. Defaults match 1.0 (100k steps, 1 MB, depth 64).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub max_steps: u32,
    pub max_output: usize,
    pub max_depth: u32,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            max_steps: MAX_STEPS,
            max_output: MAX_OUTPUT,
            max_depth: MAX_DEPTH,
        }
    }
}

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
    pub parts_by_channel: Option<HashMap<String, Vec<OutputPart>>>,
    pub choices: Option<Vec<Choice>>,
    pub write_source: PartSource,
    pub write_table: Option<String>,
    pub budget: Budget,
    pub pronunciations: Arc<HashMap<String, String>>,
    pub notes: Vec<String>,
    pub unresolved: Vec<UnresolvedQuery>,
    pub capture_frames: Vec<CaptureFrame>,
}

/// One `[capture]` frame: text plus optional lineage.
#[derive(Debug, Clone, Default)]
pub struct CaptureFrame {
    pub text: String,
    pub parts: Vec<OutputPart>,
    pub will_emit: bool,
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Captured {
    pub text: String,
    pub parts: Vec<OutputPart>,
}

impl Captured {
    pub fn append(&mut self, other: Captured) {
        self.text.push_str(&other.text);
        self.parts.extend(other.parts);
    }

    pub fn push_glue(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.text.push_str(text);
        if let Some(last) = self.parts.last_mut() {
            if last.source == PartSource::Glue && last.table.is_none() {
                last.text.push_str(text);
                return;
            }
        }
        self.parts.push(OutputPart {
            text: text.to_string(),
            source: PartSource::Glue,
            table: None,
        });
    }

    pub fn append_slice(&mut self, src: &Captured, start: usize, end: usize) {
        if start >= end || end > src.text.len() {
            return;
        }
        self.text.push_str(&src.text[start..end]);
        if src.parts.is_empty() {
            return;
        }
        self.parts
            .extend(crate::output::slice_parts(&src.parts, start, end));
    }

    pub fn trim_start(mut self) -> Self {
        let trimmed = self.text.trim_start();
        let drop = self.text.len() - trimmed.len();
        if drop == 0 {
            return self;
        }
        self.text = trimmed.to_string();
        if !self.parts.is_empty() {
            self.parts = crate::output::slice_parts(&self.parts, drop, drop + self.text.len());
        }
        self
    }
}

impl Context {
    pub fn with_budget(
        rng: Rng,
        case_mode: CaseMode,
        dictionary: Arc<Dictionary>,
        budget: Budget,
    ) -> Self {
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
            parts_by_channel: None,
            choices: None,
            write_source: PartSource::Glue,
            write_table: None,
            budget,
            pronunciations: Arc::new(HashMap::new()),
            notes: Vec::new(),
            unresolved: Vec::new(),
            capture_frames: Vec::new(),
        }
    }

    pub fn emit_target(&self) -> (bool, Option<String>) {
        match self.capture_frames.last() {
            Some(frame) => (frame.will_emit, frame.channel.clone()),
            None => (true, Some(self.channel.clone())),
        }
    }

    pub fn set_write_glue(&mut self) {
        self.write_source = PartSource::Glue;
        self.write_table = None;
    }

    pub fn set_write_dictionary(&mut self, table: &str) {
        self.write_source = PartSource::Dictionary;
        self.write_table = Some(table.to_string());
    }

    pub fn tick(&mut self, span: Span) -> Result<(), Error> {
        self.steps += 1;
        let cap = self.budget.max_steps;
        if self.steps > cap {
            return Err(Error::budget(format!(
                "exceeded {cap} steps (at {}..{})",
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
        if next > self.budget.max_output {
            return Err(Error::budget(format!(
                "exceeded {} output bytes",
                self.budget.max_output
            )));
        }
        if let Some(frame) = self.capture_frames.last_mut() {
            frame.text.push_str(s);
            if self.parts_by_channel.is_some() {
                merge_part(
                    &mut frame.parts,
                    s,
                    self.write_source,
                    self.write_table.clone(),
                );
            }
            return Ok(());
        }
        self.write_channel(&self.channel.clone(), s)?;
        Ok(())
    }

    pub fn write_channel(&mut self, name: &str, s: &str) -> Result<(), Error> {
        if s.is_empty() {
            return Ok(());
        }
        let key = if name.is_empty() { "main" } else { name };
        let chunk = if key == "main" {
            s.to_string()
        } else {
            apply_case(s, self.case_mode)
        };
        let next = self.channels.values().map(String::len).sum::<usize>() + chunk.len();
        if next > self.budget.max_output {
            return Err(Error::budget(format!(
                "exceeded {} output bytes",
                self.budget.max_output
            )));
        }
        self.channels
            .entry(key.to_string())
            .or_default()
            .push_str(&chunk);
        self.record_part(key, &chunk);
        Ok(())
    }

    fn record_part(&mut self, channel: &str, s: &str) {
        let Some(map) = self.parts_by_channel.as_mut() else {
            return;
        };
        let parts = map.entry(channel.to_string()).or_default();
        merge_part(parts, s, self.write_source, self.write_table.clone());
    }

    pub fn emit_captured(&mut self, cap: Captured) -> Result<(), Error> {
        if !self.capture_frames.is_empty() {
            return self.emit_into_frame(cap);
        }
        self.emit_to_channel(&self.channel.clone(), cap)
    }

    pub fn emit_to_channel(&mut self, name: &str, mut cap: Captured) -> Result<(), Error> {
        if !self.capture_frames.is_empty() {
            return self.emit_into_frame(cap);
        }
        if cap.text.is_empty() && cap.parts.is_empty() {
            return Ok(());
        }
        let key = if name.is_empty() { "main" } else { name };
        if key != "main" {
            let chunk = apply_case(&cap.text, self.case_mode);
            if !cap.parts.is_empty() {
                crate::output::rewrite_part_texts(&mut cap.parts, &chunk);
            }
            cap.text = chunk;
        }
        let next = self.channels.values().map(String::len).sum::<usize>() + cap.text.len();
        if next > self.budget.max_output {
            return Err(Error::budget(format!(
                "exceeded {} output bytes",
                self.budget.max_output
            )));
        }
        self.channels
            .entry(key.to_string())
            .or_default()
            .push_str(&cap.text);
        if let Some(map) = self.parts_by_channel.as_mut() {
            let dest = map.entry(key.to_string()).or_default();
            if cap.parts.is_empty() {
                merge_part(dest, &cap.text, PartSource::Glue, None);
            } else {
                for part in cap.parts {
                    merge_part(dest, &part.text, part.source, part.table);
                }
            }
        }
        Ok(())
    }

    fn emit_into_frame(&mut self, cap: Captured) -> Result<(), Error> {
        if cap.parts.is_empty() {
            self.set_write_glue();
            return self.write(&cap.text);
        }
        for part in cap.parts {
            self.write_source = part.source;
            self.write_table = part.table;
            self.write(&part.text)?;
        }
        Ok(())
    }

    fn output_len(&self) -> usize {
        if let Some(last) = self.capture_frames.last() {
            last.text.len()
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

pub fn capture<F>(ctx: &mut Context, f: F) -> Result<Captured, Error>
where
    F: FnOnce(&mut Context) -> Result<(), Error>,
{
    capture_ex(ctx, false, None, f)
}

pub fn capture_ex<F>(
    ctx: &mut Context,
    will_emit: bool,
    channel: Option<String>,
    f: F,
) -> Result<Captured, Error>
where
    F: FnOnce(&mut Context) -> Result<(), Error>,
{
    ctx.capture_frames.push(CaptureFrame {
        text: String::new(),
        parts: Vec::new(),
        will_emit,
        channel,
    });
    f(ctx)?;
    let frame = ctx.capture_frames.pop().unwrap_or_default();
    Ok(Captured {
        text: frame.text,
        parts: frame.parts,
    })
}

fn merge_part(parts: &mut Vec<OutputPart>, s: &str, source: PartSource, table: Option<String>) {
    if s.is_empty() {
        return;
    }
    if let Some(last) = parts.last_mut() {
        if last.source == source && last.table == table {
            last.text.push_str(s);
            return;
        }
    }
    parts.push(OutputPart {
        text: s.to_string(),
        source,
        table,
    });
}
