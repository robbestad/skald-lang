#![allow(clippy::too_many_arguments)]

use skald::{
    Capabilities, CaseMode, Options, Program, Seed, compile as compile_pattern, from_json,
    lint_story, parse, preflight_errors, skald,
};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

fn js_err(err: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&err.to_string())
}

fn seed_of(seed: Option<String>) -> Result<Option<Seed>, JsValue> {
    match seed {
        None => Ok(None),
        Some(s) => Seed::parse(&s).map(Some).map_err(js_err),
    }
}

fn case_of(case_mode: Option<String>) -> Option<CaseMode> {
    case_mode.map(|s| CaseMode::parse(&s))
}

fn options(
    dict: &Arc<skald::Dictionary>,
    seed: Option<String>,
    nsfw: bool,
    case_mode: Option<String>,
    story: bool,
    max_steps: Option<u32>,
    max_output: Option<u32>,
    max_depth: Option<u32>,
    capabilities: Option<Capabilities>,
    locale: Option<String>,
) -> Result<Options, JsValue> {
    let mut opts = Options {
        seed: seed_of(seed)?,
        case_mode: case_of(case_mode),
        nsfw,
        dictionary: Some(Arc::clone(dict)),
        story,
        merge: false,
        capabilities,
        locale,
        ..Default::default()
    };
    if let Some(n) = max_steps {
        opts.budget.max_steps = n;
    }
    if let Some(n) = max_output {
        opts.budget.max_output = n as usize;
    }
    if let Some(n) = max_depth {
        opts.budget.max_depth = n;
    }
    Ok(opts)
}

/// WASM engine. Construct with dictionary JSON (`{"tables":{...}}`).
#[wasm_bindgen]
pub struct Engine {
    dict: Arc<skald::Dictionary>,
    locale: Option<String>,
    capabilities: Option<Capabilities>,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new(dict_json: &str) -> Result<Engine, JsValue> {
        let json = if dict_json.trim().is_empty() {
            "{\"tables\":{}}"
        } else {
            dict_json
        };
        let dict = from_json(json).map_err(js_err)?;
        Ok(Engine {
            dict: Arc::new(dict),
            locale: None,
            capabilities: None,
        })
    }

    #[wasm_bindgen(js_name = fromLanguagePack)]
    pub fn from_language_pack(json: &str) -> Result<Engine, JsValue> {
        let pack = skald::from_language_pack(json).map_err(js_err)?;
        Ok(Engine {
            dict: Arc::new(pack.dictionary),
            locale: Some(pack.locale),
            capabilities: Some(pack.capabilities),
        })
    }

    #[wasm_bindgen(js_name = locale)]
    pub fn locale(&self) -> Option<String> {
        self.locale.clone()
    }

    pub fn preflight(&self, pattern: &str) -> Result<(), JsValue> {
        preflight_errors(pattern, &self.dict, self.capabilities.as_ref()).map_err(js_err)
    }

    pub fn overlay(&self, extra_json: &str) -> Result<Engine, JsValue> {
        let extra = from_json(extra_json).map_err(js_err)?;
        let mut dict = (*self.dict).clone();
        dict.overlay(&extra);
        Ok(Engine {
            dict: Arc::new(dict),
            locale: self.locale.clone(),
            capabilities: self.capabilities.clone(),
        })
    }

    pub fn run(
        &self,
        pattern: &str,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
    ) -> Result<String, JsValue> {
        self.run_full(pattern, seed, nsfw, case_mode, false, None, None, None)
    }

    #[wasm_bindgen(js_name = runFull)]
    pub fn run_full(
        &self,
        pattern: &str,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
        story: bool,
        max_steps: Option<u32>,
        max_output: Option<u32>,
        max_depth: Option<u32>,
    ) -> Result<String, JsValue> {
        skald(
            pattern,
            &options(
                &self.dict,
                seed,
                nsfw,
                case_mode,
                story,
                max_steps,
                max_output,
                max_depth,
                self.capabilities.clone(),
                self.locale.clone(),
            )?,
        )
        .map_err(js_err)
    }

    pub fn run_output(
        &self,
        pattern: &str,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
    ) -> Result<String, JsValue> {
        self.output_full(pattern, seed, nsfw, case_mode, false, None, None, None)
    }

    #[wasm_bindgen(js_name = outputFull)]
    pub fn output_full(
        &self,
        pattern: &str,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
        story: bool,
        max_steps: Option<u32>,
        max_output: Option<u32>,
        max_depth: Option<u32>,
    ) -> Result<String, JsValue> {
        skald::skald_output(
            pattern,
            &options(
                &self.dict,
                seed,
                nsfw,
                case_mode,
                story,
                max_steps,
                max_output,
                max_depth,
                self.capabilities.clone(),
                self.locale.clone(),
            )?,
        )
        .map(|o| o.to_json())
        .map_err(js_err)
    }

    pub fn explain(
        &self,
        pattern: &str,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
    ) -> Result<String, JsValue> {
        self.explain_full(pattern, seed, nsfw, case_mode, false, None, None, None)
    }

    #[wasm_bindgen(js_name = explainFull)]
    pub fn explain_full(
        &self,
        pattern: &str,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
        story: bool,
        max_steps: Option<u32>,
        max_output: Option<u32>,
        max_depth: Option<u32>,
    ) -> Result<String, JsValue> {
        skald::explain(
            pattern,
            &options(
                &self.dict,
                seed,
                nsfw,
                case_mode,
                story,
                max_steps,
                max_output,
                max_depth,
                self.capabilities.clone(),
                self.locale.clone(),
            )?,
        )
        .map(|o| o.to_json())
        .map_err(js_err)
    }

    pub fn story_lint(&self, pattern: &str) -> Result<String, JsValue> {
        let ast = parse(pattern).map_err(js_err)?;
        let notes: Vec<String> = lint_story(pattern, &ast)
            .iter()
            .map(|d| d.to_note())
            .collect();
        Ok(notes_json(&notes))
    }

    pub fn compile(&self, pattern: &str) -> Result<Compiled, JsValue> {
        Ok(Compiled {
            program: compile_pattern(pattern).map_err(js_err)?,
            dict: Arc::clone(&self.dict),
            capabilities: self.capabilities.clone(),
            locale: self.locale.clone(),
        })
    }
}

#[wasm_bindgen]
pub struct Compiled {
    program: Program,
    dict: Arc<skald::Dictionary>,
    capabilities: Option<Capabilities>,
    locale: Option<String>,
}

#[wasm_bindgen]
impl Compiled {
    pub fn run(
        &self,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
    ) -> Result<String, JsValue> {
        self.run_full(seed, nsfw, case_mode, false, None, None, None)
    }

    #[wasm_bindgen(js_name = runFull)]
    pub fn run_full(
        &self,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
        story: bool,
        max_steps: Option<u32>,
        max_output: Option<u32>,
        max_depth: Option<u32>,
    ) -> Result<String, JsValue> {
        self.program
            .run(&options(
                &self.dict,
                seed,
                nsfw,
                case_mode,
                story,
                max_steps,
                max_output,
                max_depth,
                self.capabilities.clone(),
                self.locale.clone(),
            )?)
            .map_err(js_err)
    }

    pub fn run_output(
        &self,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
    ) -> Result<String, JsValue> {
        self.output_full(seed, nsfw, case_mode, false, None, None, None)
    }

    #[wasm_bindgen(js_name = outputFull)]
    pub fn output_full(
        &self,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
        story: bool,
        max_steps: Option<u32>,
        max_output: Option<u32>,
        max_depth: Option<u32>,
    ) -> Result<String, JsValue> {
        self.program
            .run_output(&options(
                &self.dict,
                seed,
                nsfw,
                case_mode,
                story,
                max_steps,
                max_output,
                max_depth,
                self.capabilities.clone(),
                self.locale.clone(),
            )?)
            .map(|o| o.to_json())
            .map_err(js_err)
    }

    pub fn explain(
        &self,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
    ) -> Result<String, JsValue> {
        self.explain_full(seed, nsfw, case_mode, false, None, None, None)
    }

    #[wasm_bindgen(js_name = explainFull)]
    pub fn explain_full(
        &self,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
        story: bool,
        max_steps: Option<u32>,
        max_output: Option<u32>,
        max_depth: Option<u32>,
    ) -> Result<String, JsValue> {
        self.program
            .explain(&options(
                &self.dict,
                seed,
                nsfw,
                case_mode,
                story,
                max_steps,
                max_output,
                max_depth,
                self.capabilities.clone(),
                self.locale.clone(),
            )?)
            .map(|o| o.to_json())
            .map_err(js_err)
    }
}

fn notes_json(notes: &[String]) -> String {
    let mut out = String::from("[");
    for (i, note) in notes.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        for c in note.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                c => out.push(c),
            }
        }
        out.push('"');
    }
    out.push(']');
    out
}
