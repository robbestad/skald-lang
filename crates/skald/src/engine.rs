use crate::ast::{CaseMode, Node};
use crate::dict::{Dictionary, en_us};
use crate::error::Error;
use crate::interpret::interpret_output;
use crate::output::Output;
use crate::parse::parse;
use crate::rng::{Rng, Seed};
use crate::runtime::Context;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub seed: Option<Seed>,
    pub case_mode: Option<CaseMode>,
    pub nsfw: bool,
    /// `None` uses the bundled English dictionary.
    pub dictionary: Option<Arc<Dictionary>>,
}

#[derive(Debug, Clone)]
pub struct Program {
    ast: Vec<Node>,
}

impl Program {
    pub fn compile(pattern: &str) -> Result<Self, Error> {
        Ok(Self {
            ast: parse(pattern)?,
        })
    }

    pub fn run(&self, opts: &Options) -> Result<String, Error> {
        Ok(self.run_output(opts)?.text)
    }

    pub fn run_output(&self, opts: &Options) -> Result<Output, Error> {
        self.run_ctx(opts, false)
    }

    pub fn explain(&self, opts: &Options) -> Result<Output, Error> {
        self.run_ctx(opts, true)
    }

    fn run_ctx(&self, opts: &Options, trace: bool) -> Result<Output, Error> {
        let rng = Rng::from_seed(opts.seed.clone());
        let case = opts.case_mode.unwrap_or(CaseMode::Default);
        let dict = opts.dictionary.clone().unwrap_or_else(en_us);
        let mut ctx = Context::new(rng, case, dict);
        ctx.nsfw = opts.nsfw;
        if trace {
            ctx.picks = Some(Vec::new());
        }
        interpret_output(&self.ast, &mut ctx)
    }
}

pub fn compile(pattern: &str) -> Result<Program, Error> {
    Program::compile(pattern)
}

pub fn skald(pattern: &str, opts: &Options) -> Result<String, Error> {
    compile(pattern)?.run(opts)
}

pub fn skald_output(pattern: &str, opts: &Options) -> Result<Output, Error> {
    compile(pattern)?.run_output(opts)
}

pub fn explain(pattern: &str, opts: &Options) -> Result<Output, Error> {
    compile(pattern)?.explain(opts)
}
