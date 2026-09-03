use skald::{
    CaseMode, Options, Program, Seed, compile as compile_pattern, from_json, lint_story, parse,
    skald,
};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

fn js_err(err: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&err.to_string())
}

fn seed_of(seed: Option<String>) -> Option<Seed> {
    seed.map(|s| Seed::parse(&s))
}

fn case_of(case_mode: Option<String>) -> Option<CaseMode> {
    case_mode.map(|s| CaseMode::parse(&s))
}

fn options(
    dict: &Arc<skald::Dictionary>,
    seed: Option<String>,
    nsfw: bool,
    case_mode: Option<String>,
) -> Options {
    Options {
        seed: seed_of(seed),
        case_mode: case_of(case_mode),
        nsfw,
        dictionary: Some(Arc::clone(dict)),
        ..Default::default()
    }
}

/// WASM engine. Construct with dictionary JSON (`{"tables":{...}}`).
#[wasm_bindgen]
pub struct Engine {
    dict: Arc<skald::Dictionary>,
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
        })
    }

    pub fn run(
        &self,
        pattern: &str,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
    ) -> Result<String, JsValue> {
        skald(pattern, &options(&self.dict, seed, nsfw, case_mode)).map_err(js_err)
    }

    pub fn run_output(
        &self,
        pattern: &str,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
    ) -> Result<String, JsValue> {
        skald::skald_output(pattern, &options(&self.dict, seed, nsfw, case_mode))
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
        skald::explain(pattern, &options(&self.dict, seed, nsfw, case_mode))
            .map(|o| o.to_json())
            .map_err(js_err)
    }

    pub fn story_lint(&self, pattern: &str) -> Result<String, JsValue> {
        let ast = parse(pattern).map_err(js_err)?;
        Ok(notes_json(&lint_story(pattern, &ast)))
    }

    pub fn compile(&self, pattern: &str) -> Result<Compiled, JsValue> {
        Ok(Compiled {
            program: compile_pattern(pattern).map_err(js_err)?,
            dict: Arc::clone(&self.dict),
        })
    }
}

#[wasm_bindgen]
pub struct Compiled {
    program: Program,
    dict: Arc<skald::Dictionary>,
}

#[wasm_bindgen]
impl Compiled {
    pub fn run(
        &self,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
    ) -> Result<String, JsValue> {
        self.program
            .run(&options(&self.dict, seed, nsfw, case_mode))
            .map_err(js_err)
    }

    pub fn run_output(
        &self,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
    ) -> Result<String, JsValue> {
        self.program
            .run_output(&options(&self.dict, seed, nsfw, case_mode))
            .map(|o| o.to_json())
            .map_err(js_err)
    }

    pub fn explain(
        &self,
        seed: Option<String>,
        nsfw: bool,
        case_mode: Option<String>,
    ) -> Result<String, JsValue> {
        self.program
            .explain(&options(&self.dict, seed, nsfw, case_mode))
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
