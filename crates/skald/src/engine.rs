use crate::ast::{CaseMode, Node};
use crate::dict::{Capabilities, Dictionary, en_us};
use crate::error::Error;
use crate::interpret::interpret_output;
use crate::output::Output;
use crate::parse::parse;
use crate::rng::{Rng, Seed};
use crate::runtime::{Budget, Context};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Options {
    pub seed: Option<Seed>,
    pub case_mode: Option<CaseMode>,
    pub nsfw: bool,
    /// `None` uses the bundled English dictionary.
    pub dictionary: Option<Arc<Dictionary>>,
    /// Step / output / depth caps. Defaults are 100k / 1 MB / 64.
    pub budget: Budget,
    /// Extra X-SAMPA pronunciations keyed by lowercase surface form.
    /// Used when a dictionary row has no `| pron` and a rhyme query needs one.
    pub pronunciations: Option<Arc<HashMap<String, String>>>,
    /// When true, story-lint diagnostics are added to `explain` and `output`.
    pub story: bool,
    /// When `dictionary` is set, merge it over bundled English (replace same names).
    /// `false` uses only the provided dictionary. Ignored when `dictionary` is `None`.
    /// Default is `true`.
    pub merge: bool,
    /// Language-pack capability profile. `None` is unrestricted (legacy English).
    pub capabilities: Option<Capabilities>,
    /// Requested locale. Pack-backed runs must match the pack locale.
    pub locale: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            seed: None,
            case_mode: None,
            nsfw: false,
            dictionary: None,
            budget: Budget::default(),
            pronunciations: None,
            story: false,
            merge: true,
            capabilities: None,
            locale: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    ast: Vec<Node>,
    source: String,
}

impl Program {
    pub fn compile(pattern: &str) -> Result<Self, Error> {
        Ok(Self {
            ast: parse(pattern)?,
            source: pattern.to_string(),
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
        let dict = resolve_dictionary(opts);
        if opts.capabilities.is_some() {
            crate::preflight::preflight_errors(
                &self.source,
                dict.as_ref(),
                opts.capabilities.as_ref(),
            )?;
        }
        if matches!(case, CaseMode::Title) {
            if let Some(caps) = &opts.capabilities {
                if !caps.allows_title_case() {
                    return Err(Error::runtime(
                        "title case is not supported by this language pack",
                        None,
                    ));
                }
            }
        }
        let mut ctx = Context::with_budget(rng, case, dict, opts.budget);
        ctx.nsfw = opts.nsfw;
        ctx.capabilities = opts.capabilities.clone();
        if let Some(pron) = &opts.pronunciations {
            ctx.pronunciations = Arc::clone(pron);
        }
        if trace {
            ctx.picks = Some(Vec::new());
            ctx.parts_by_channel = Some(HashMap::new());
            ctx.choices = Some(Vec::new());
        }
        let mut out = interpret_output(&self.ast, &mut ctx)?;
        if opts.story {
            let diagnostics = crate::story::lint_story(&self.source, &self.ast);
            out.notes.extend(diagnostics.iter().map(|d| d.to_note()));
            out.diagnostics.extend(diagnostics);
        }
        Ok(out)
    }
}

fn resolve_dictionary(opts: &Options) -> Arc<Dictionary> {
    match &opts.dictionary {
        None => en_us(),
        Some(extra) if opts.merge => {
            let mut base = (*en_us()).clone();
            base.overlay(extra);
            Arc::new(base)
        }
        Some(only) => Arc::clone(only),
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
