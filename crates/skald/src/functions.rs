use crate::ast::{Node, TagNode};
use crate::error::Error;
use crate::format::case::parse_case_mode;
use crate::format::number::format_number;
use crate::parse::decode_sep_arg;
use crate::runtime::{BlockAttrs, Context, FnPending, MAX_DEPTH, Rep, UserFn, capture};
use crate::sync::{SyncState, parse_sync_type};
use crate::value::Value;

pub type EvalSeq = fn(&[Node], &mut Context) -> Result<(), Error>;
pub type EvalExpr = fn(&[Node], &mut Context) -> Result<Value, Error>;

const RESERVED: &[&str] = &[
    "a", "caps", "case", "chance", "collect", "fn", "if", "index", "index1", "i", "i1", "join",
    "len", "let", "n", "num", "numfmt", "out", "pick", "protect", "r", "rep", "repeach", "repnum",
    "rhyme", "rn", "rs", "s", "sep", "sync", "x", "xdel",
];

pub fn is_reserved(name: &str) -> bool {
    RESERVED.contains(&name)
}

pub fn reserved_names() -> &'static [&'static str] {
    RESERVED
}

pub fn run_tag(
    tag: &TagNode,
    ctx: &mut Context,
    eval_sequence: EvalSeq,
    eval_expr: EvalExpr,
    as_expr: bool,
) -> Result<Value, Error> {
    let name = tag.name.trim_start_matches(':');
    let a = &tag.args;
    let greedy = |nodes: Option<&[Node]>, ctx: &mut Context| -> Result<String, Error> {
        match nodes {
            Some(n) if !n.is_empty() => capture(ctx, |c| eval_sequence(n, c)),
            _ => Ok(String::new()),
        }
    };
    let num = |nodes: Option<&[Node]>, fallback: f64, ctx: &mut Context| -> Result<f64, Error> {
        let s = greedy(nodes, ctx)?;
        Ok(s.trim().parse::<f64>().unwrap_or(fallback))
    };

    match name {
        "case" | "caps" => {
            let raw = if tag.arg.is_empty() {
                greedy(a.first().map(|v| v.as_slice()), ctx)?
            } else {
                tag.arg.clone()
            };
            ctx.case_mode = parse_case_mode(&raw);
            Ok(Value::Nil)
        }
        "rep" | "r" => {
            let n = num(a.first().map(|v| v.as_slice()), 1.0, ctx)?.trunc() as i64;
            ctx.attrs.rep = Rep::Times(if n > 0 { n } else { 1 });
            Ok(Value::Nil)
        }
        "repeach" => {
            ctx.attrs.rep = Rep::Each;
            Ok(Value::Nil)
        }
        "sep" | "s" => {
            ctx.attrs.sep = a.first().map(|v| sep_nodes(v.clone()));
            Ok(Value::Nil)
        }
        "rs" => {
            let n = num(a.first().map(|v| v.as_slice()), 1.0, ctx)?.trunc() as i64;
            ctx.attrs.rep = Rep::Times(if n > 0 { n } else { 1 });
            ctx.attrs.sep = a.get(1).map(|v| sep_nodes(v.clone()));
            Ok(Value::Nil)
        }
        "a" => {
            ctx.pending_article = true;
            Ok(Value::Nil)
        }
        "chance" => {
            ctx.attrs.chance = num(a.first().map(|v| v.as_slice()), 100.0, ctx)?;
            Ok(Value::Nil)
        }
        "n" | "num" => {
            let n = if a.len() >= 2 {
                let min = num(Some(&a[0]), 0.0, ctx)?;
                let max = num(Some(&a[1]), min, ctx)?;
                let lo = min.min(max);
                let hi = max.max(min);
                let span = ((hi - lo).floor() as i64 + 1).max(1);
                lo as i64 + ctx.rng.int(span)
            } else {
                let raw = greedy(a.first().map(|v| v.as_slice()), ctx)?;
                if let Ok(v) = raw.trim().parse::<f64>() {
                    v.trunc() as i64
                } else {
                    if !as_expr {
                        ctx.write(&raw)?;
                    }
                    return Ok(Value::Str(raw));
                }
            };
            ctx.last_number = Some(n);
            if !as_expr {
                ctx.write(&format_number(n, &ctx.numfmt))?;
            }
            Ok(Value::Int(n))
        }
        "numfmt" => {
            let mode = greedy(a.first().map(|v| v.as_slice()), ctx)?
                .trim()
                .to_ascii_lowercase();
            if let Some(body) = a.get(1) {
                let prev = ctx.numfmt.clone();
                ctx.numfmt = mode;
                eval_sequence(body, ctx)?;
                ctx.numfmt = prev;
            } else {
                ctx.numfmt = mode;
            }
            Ok(Value::Nil)
        }
        "rn" | "repnum" | "index1" | "i1" => {
            let v = ctx.rep_index + 1;
            ctx.last_number = Some(v);
            if !as_expr {
                ctx.write(&format_number(v, &ctx.numfmt))?;
            }
            Ok(Value::Int(v))
        }
        "index" | "i" => {
            let v = ctx.rep_index;
            ctx.last_number = Some(v);
            if !as_expr {
                ctx.write(&format_number(v, &ctx.numfmt))?;
            }
            Ok(Value::Int(v))
        }
        "x" | "sync" => {
            let id = greedy(a.first().map(|v| v.as_slice()), ctx)?
                .trim()
                .to_string();
            let ty = greedy(a.get(1).map(|v| v.as_slice()), ctx)?;
            let kind = parse_sync_type(&ty);
            ctx.syncs
                .entry(id.clone())
                .and_modify(|s| s.kind = kind)
                .or_insert_with(|| SyncState::new(kind));
            ctx.attrs.sync = Some(id);
            Ok(Value::Nil)
        }
        "xdel" => {
            let id = greedy(a.first().map(|v| v.as_slice()), ctx)?;
            ctx.syncs.remove(id.trim());
            Ok(Value::Nil)
        }
        "protect" => {
            let saved = ctx.attrs.clone();
            ctx.attrs = BlockAttrs::default();
            eval_sequence(a.first().map(|v| v.as_slice()).unwrap_or(&[]), ctx)?;
            ctx.attrs = saved;
            Ok(Value::Nil)
        }
        "if" => {
            if a.len() >= 2 {
                let cond = greedy(Some(&a[0]), ctx)?;
                let cond = cond.trim();
                let ok = cond == "true"
                    || cond == "1"
                    || ctx.match_carriers.contains_key(cond)
                    || ctx.lookup_binding(cond).is_some();
                let body = if ok {
                    &a[1]
                } else {
                    a.get(2).map(|v| v.as_slice()).unwrap_or(&[])
                };
                eval_sequence(body, ctx)?;
            } else {
                ctx.pending_if = Some(tag.arg.clone());
            }
            Ok(Value::Nil)
        }
        "let" => {
            let name = eval_expr(a.first().map(|v| v.as_slice()).unwrap_or(&[]), ctx)?
                .to_print()
                .trim()
                .to_string();
            if name.is_empty() {
                return Err(Error::runtime("[let] needs a name", Some(tag.span)));
            }
            if is_reserved(&name) {
                return Err(Error::runtime(
                    format!("[let] cannot bind reserved name '{name}'"),
                    Some(tag.span),
                ));
            }
            let value = eval_expr(a.get(1).map(|v| v.as_slice()).unwrap_or(&[]), ctx)?;
            ctx.bind(name, value.clone());
            Ok(value)
        }
        "fn" => {
            let name = eval_expr(a.first().map(|v| v.as_slice()).unwrap_or(&[]), ctx)?
                .to_print()
                .trim()
                .to_string();
            if name.is_empty() {
                return Err(Error::runtime("[fn] needs a name", Some(tag.span)));
            }
            if is_reserved(&name) {
                return Err(Error::runtime(
                    format!("[fn] cannot define reserved name '{name}'"),
                    Some(tag.span),
                ));
            }
            let mut params = Vec::new();
            for arg in a.iter().skip(1) {
                let p = eval_expr(arg, ctx)?.to_print().trim().to_string();
                if p.is_empty() {
                    continue;
                }
                if is_reserved(&p) {
                    return Err(Error::runtime(
                        format!("[fn] parameter '{p}' is reserved"),
                        Some(tag.span),
                    ));
                }
                params.push(p);
            }
            ctx.attrs.fn_def = Some(FnPending { name, params });
            Ok(Value::Nil)
        }
        "out" => {
            let name = eval_expr(a.first().map(|v| v.as_slice()).unwrap_or(&[]), ctx)?
                .to_print()
                .trim()
                .to_string();
            if name.is_empty() {
                return Err(Error::runtime("[out] needs a channel name", Some(tag.span)));
            }
            ctx.attrs.out = Some(name);
            Ok(Value::Nil)
        }
        "rhyme" => {
            let raw = if tag.arg.is_empty() {
                greedy(a.first().map(|v| v.as_slice()), ctx)?
            } else {
                tag.arg.clone()
            };
            let Some(mode) = crate::rhyme::RhymeMode::parse(&raw) else {
                let hint =
                    crate::error::did_you_mean(raw.trim(), &["perfect", "slant", "alliteration"]);
                let extra = match hint {
                    Some(h) => format!(". Did you mean '{h}'?"),
                    None => " (perfect, slant, alliteration)".to_string(),
                };
                return Err(Error::runtime(
                    format!("unknown rhyme mode '{raw}'{extra}"),
                    Some(tag.span),
                ));
            };
            ctx.rhyme_mode = mode;
            Ok(Value::Nil)
        }
        "collect" => {
            let n = num(a.first().map(|v| v.as_slice()), 0.0, ctx)?.trunc() as i64;
            let n = if n < 0 { 0 } else { n as usize };
            let body = a.get(1).map(|v| v.as_slice()).unwrap_or(&[]);
            let saved = ctx.rep_index;
            let mut items = Vec::with_capacity(n);
            for i in 0..n {
                ctx.rep_index = i as i64;
                items.push(eval_expr(body, ctx)?);
            }
            ctx.rep_index = saved;
            Ok(Value::List(items))
        }
        "join" => {
            let src = eval_expr(a.first().map(|v| v.as_slice()).unwrap_or(&[]), ctx)?;
            let items = resolve_list(src, ctx);
            let sep = greedy(a.get(1).map(|v| v.as_slice()), ctx)?
                .trim_start()
                .to_string();
            let conj = greedy(a.get(2).map(|v| v.as_slice()), ctx)?;
            let text = oxford_join(&items, &sep, conj.trim());
            if !as_expr {
                ctx.write(&text)?;
            }
            Ok(Value::Str(text))
        }
        "len" => {
            let src = eval_expr(a.first().map(|v| v.as_slice()).unwrap_or(&[]), ctx)?;
            let v = deref_value(src, ctx);
            let n = v.len();
            ctx.last_number = Some(n);
            if !as_expr {
                ctx.write(&format_number(n, &ctx.numfmt))?;
            }
            Ok(Value::Int(n))
        }
        "pick" => {
            let src = eval_expr(a.first().map(|v| v.as_slice()).unwrap_or(&[]), ctx)?;
            let items = resolve_list(src, ctx);
            if items.is_empty() {
                return Ok(Value::Nil);
            }
            let item = ctx.rng.pick(&items).cloned().unwrap_or(Value::Nil);
            if !as_expr {
                ctx.write(&item.to_print())?;
            }
            Ok(item)
        }
        _ => {
            if let Some(value) = ctx.lookup_binding(name).cloned() {
                if !as_expr {
                    ctx.write(&value.to_print())?;
                }
                return Ok(value);
            }
            if let Some(func) = ctx.functions.get(name).cloned() {
                return call_user_fn(func, a, ctx, eval_sequence, eval_expr, as_expr);
            }
            let mut names: Vec<&str> = reserved_names().to_vec();
            for frame in &ctx.bindings {
                for key in frame.keys() {
                    names.push(key.as_str());
                }
            }
            for key in ctx.functions.keys() {
                names.push(key.as_str());
            }
            let message = match crate::error::did_you_mean(name, &names) {
                Some(hint) => format!("Unknown tag: [{name}]. Did you mean [{hint}]?"),
                None => format!("Unknown tag: [{name}]"),
            };
            Err(Error::runtime(message, Some(tag.span)))
        }
    }
}

fn call_user_fn(
    func: UserFn,
    args: &[Vec<Node>],
    ctx: &mut Context,
    eval_sequence: EvalSeq,
    eval_expr: EvalExpr,
    as_expr: bool,
) -> Result<Value, Error> {
    if ctx.call_depth >= MAX_DEPTH {
        return Err(Error::budget(format!(
            "function call depth exceeded ({MAX_DEPTH})"
        )));
    }
    let mut vals = Vec::with_capacity(func.params.len());
    for i in 0..func.params.len() {
        vals.push(eval_expr(
            args.get(i).map(|v| v.as_slice()).unwrap_or(&[]),
            ctx,
        )?);
    }
    ctx.call_depth += 1;
    ctx.push_frame();
    for (param, val) in func.params.iter().zip(vals) {
        ctx.bind(param.clone(), val);
    }
    let result = if as_expr {
        eval_expr(&func.body, ctx)
    } else {
        eval_sequence(&func.body, ctx).map(|()| Value::Nil)
    };
    ctx.pop_frame();
    ctx.call_depth -= 1;
    result
}

fn deref_value(v: Value, ctx: &Context) -> Value {
    if let Value::Str(s) = &v {
        let t = s.trim();
        if let Some(bound) = ctx.lookup_binding(t) {
            return bound.clone();
        }
    }
    v
}

fn resolve_list(v: Value, ctx: &Context) -> Vec<Value> {
    match deref_value(v, ctx) {
        Value::List(items) => items,
        Value::Nil => Vec::new(),
        other => vec![other],
    }
}

fn oxford_join(items: &[Value], sep: &str, conj: &str) -> String {
    let parts: Vec<String> = items.iter().map(Value::to_print).collect();
    match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        2 if conj.is_empty() => format!("{}{}{}", parts[0], sep, parts[1]),
        2 => format!("{} {conj} {}", parts[0], parts[1]),
        n => {
            let head = parts[..n - 1].join(sep);
            if conj.is_empty() {
                format!("{head}{sep}{}", parts[n - 1])
            } else {
                format!("{head}{sep}{conj} {}", parts[n - 1])
            }
        }
    }
}

fn sep_nodes(nodes: Vec<Node>) -> Vec<Node> {
    if nodes.len() == 1 {
        if let Node::Text(t) = &nodes[0] {
            if t.value == " " || t.value == "\n" || t.value == "\t" {
                return nodes;
            }
            let mut t = t.clone();
            t.value = decode_sep_arg(&t.value);
            return vec![Node::Text(t)];
        }
    }
    nodes
}
