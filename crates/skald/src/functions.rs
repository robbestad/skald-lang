use crate::ast::{Node, TagNode};
use crate::error::Error;
use crate::format::case::parse_case_mode;
use crate::format::number::format_number;
use crate::parse::decode_sep_arg;
use crate::runtime::{BlockAttrs, Context, FnPending, Rep, UserFn, capture};
use crate::sync::{SyncState, parse_sync_type};
use crate::value::Value;

pub type EvalSeq = fn(&[Node], &mut Context) -> Result<(), Error>;
pub type EvalExpr = fn(&[Node], &mut Context) -> Result<Value, Error>;

const RESERVED: &[&str] = &[
    "a", "caps", "case", "chance", "collect", "fn", "if", "index", "index1", "i", "i1", "join",
    "len", "let", "n", "num", "numfmt", "out", "pick", "protect", "r", "rep", "repeach", "repnum",
    "map", "replace", "rhyme", "rn", "rs", "s", "sep", "sync", "x", "xdel",
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
            Some(n) if !n.is_empty() => Ok(capture(ctx, |c| eval_sequence(n, c))?.text),
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
            let value = bind_let_value(
                a.get(1).map(|v| v.as_slice()).unwrap_or(&[]),
                ctx,
                eval_expr,
            )?;
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
                let hint = crate::error::did_you_mean(raw.trim(), crate::rhyme::RhymeMode::names());
                let extra = match hint {
                    Some(h) => format!(". Did you mean '{h}'?"),
                    None => format!(" ({})", crate::rhyme::RhymeMode::names().join(", ")),
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
        "map" => run_map(tag, a, ctx, eval_expr),
        "replace" => run_replace(tag, a, ctx, eval_sequence, eval_expr, as_expr),
        "replacer" => Err(Error::runtime(
            "the replacer mini-language is out of scope; use [replace: input; /pat/; body]",
            Some(tag.span),
        )),
        _ => {
            if let Some(value) = ctx.lookup_binding(name).cloned() {
                return lookup_binding(value, a, ctx, eval_sequence, eval_expr, as_expr, tag.span);
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
    if ctx.call_depth >= ctx.budget.max_depth {
        return Err(Error::budget(format!(
            "function call depth exceeded ({})",
            ctx.budget.max_depth
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

fn bind_let_value(nodes: &[Node], ctx: &mut Context, eval_expr: EvalExpr) -> Result<Value, Error> {
    let trimmed = trim_nodes(nodes);
    if trimmed.len() == 1 {
        if let Node::Block(b) = &trimmed[0] {
            if b.alternatives.len() == 1 && b.alternatives[0].weight.is_none() {
                return Ok(Value::Pattern(b.alternatives[0].nodes.clone()));
            }
            return Ok(Value::Pattern(vec![Node::Block(b.clone())]));
        }
    }
    eval_expr(nodes, ctx)
}

fn trim_nodes(nodes: &[Node]) -> &[Node] {
    let start = nodes
        .iter()
        .position(|n| !matches!(n, Node::Text(t) if t.value.chars().all(char::is_whitespace)))
        .unwrap_or(nodes.len());
    let end = nodes
        .iter()
        .rposition(|n| !matches!(n, Node::Text(t) if t.value.chars().all(char::is_whitespace)))
        .map(|i| i + 1)
        .unwrap_or(0);
    if start >= end {
        &[]
    } else {
        &nodes[start..end]
    }
}

fn run_replace(
    tag: &TagNode,
    args: &[Vec<Node>],
    ctx: &mut Context,
    eval_sequence: EvalSeq,
    _eval_expr: EvalExpr,
    as_expr: bool,
) -> Result<Value, Error> {
    let input_raw = capture(ctx, |c| {
        eval_sequence(
            trim_nodes(args.first().map(|v| v.as_slice()).unwrap_or(&[])),
            c,
        )
    })?;
    let start = input_raw.text.len() - input_raw.text.trim_start().len();
    let trimmed_len = input_raw.text.trim().len();
    let mut input = crate::runtime::Captured::default();
    if start < start + trimmed_len && start + trimmed_len <= input_raw.text.len() {
        input.append_slice(&input_raw, start, start + trimmed_len);
    }
    if input.text.is_empty() {
        input.text = input_raw.text.trim().to_string();
    }
    let input_text = input.text.clone();
    let pat_raw = capture(ctx, |c| {
        eval_sequence(
            trim_nodes(args.get(1).map(|v| v.as_slice()).unwrap_or(&[])),
            c,
        )
    })?;
    let pat = strip_regex_delims(pat_raw.text.trim());
    if pat.is_empty() {
        return Err(Error::runtime(
            "[replace] needs a regex pattern",
            Some(tag.span),
        ));
    }
    let re = regex::Regex::new(&pat).map_err(|e| {
        Error::runtime(
            format!("invalid [replace] regex /{pat}/: {e}"),
            Some(tag.span),
        )
    })?;
    let body = replace_body(args.get(2).map(|v| v.as_slice()).unwrap_or(&[]), ctx);
    let mut out = crate::runtime::Captured::default();
    let mut last = 0usize;
    for caps in re.captures_iter(&input_text) {
        ctx.tick(tag.span)?;
        let m = caps.get(0).expect("full match");
        if input.parts.is_empty() {
            out.push_glue(&input_text[last..m.start()]);
        } else {
            out.append_slice(&input, last, m.start());
        }
        ctx.push_frame();
        ctx.bind("m".to_string(), Value::Str(m.as_str().to_string()));
        for i in 1..caps.len() {
            let val = caps.get(i).map(|c| c.as_str()).unwrap_or("");
            ctx.bind(format!("m{i}"), Value::Str(val.to_string()));
        }
        let piece = capture(ctx, |c| eval_sequence(&body, c))?;
        ctx.pop_frame();
        out.append(piece);
        last = m.end();
    }
    if last < input_text.len() {
        if input.parts.is_empty() {
            out.push_glue(&input_text[last..]);
        } else {
            out.append_slice(&input, last, input_text.len());
        }
    }
    let text = out.text.clone();
    if !as_expr {
        ctx.emit_captured(out)?;
    }
    Ok(Value::Str(text))
}

fn strip_regex_delims(pat: &str) -> String {
    let t = pat.trim();
    if t.len() >= 2 && t.starts_with('/') && t.ends_with('/') {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

fn replace_body(nodes: &[Node], ctx: &Context) -> Vec<Node> {
    let trimmed = trim_nodes(nodes);
    if let [Node::Text(t)] = trimmed {
        let name = t.value.trim();
        if let Some(Value::Pattern(p)) = ctx.lookup_binding(name) {
            return p.clone();
        }
    }
    if let [Node::Tag(tag)] = trimmed {
        let name = tag.name.trim_start_matches(':');
        if let Some(Value::Pattern(p)) = ctx.lookup_binding(name) {
            return p.clone();
        }
    }
    trimmed.to_vec()
}

fn lookup_binding(
    value: Value,
    args: &[Vec<Node>],
    ctx: &mut Context,
    eval_sequence: EvalSeq,
    eval_expr: EvalExpr,
    as_expr: bool,
    span: crate::span::Span,
) -> Result<Value, Error> {
    match value {
        Value::Pattern(nodes) => {
            let spread = if args.is_empty() {
                None
            } else {
                let first = eval_expr(args.first().map(|v| v.as_slice()).unwrap_or(&[]), ctx)?;
                match as_map(first, ctx) {
                    Some(pairs) => Some(pairs),
                    None => {
                        return Err(Error::runtime(
                            "a pattern takes a [map] as its argument",
                            Some(span),
                        ));
                    }
                }
            };
            eval_pattern(&nodes, spread, ctx, eval_sequence, eval_expr, as_expr)
        }
        Value::Map(pairs) => {
            if args.is_empty() {
                return Ok(Value::Map(pairs));
            }
            let mut cur = Value::Map(pairs.clone());
            let mut from = pairs;
            for arg in args {
                let key = eval_expr(arg, ctx)?.to_print().trim().to_string();
                cur = match &cur {
                    Value::Map(p) => {
                        from = p.clone();
                        map_get(p, &key).cloned().unwrap_or(Value::Nil)
                    }
                    _ => Value::Nil,
                };
            }
            match cur {
                Value::Pattern(nodes) => {
                    eval_pattern(&nodes, Some(from), ctx, eval_sequence, eval_expr, as_expr)
                }
                other => write_bound(other, ctx, as_expr),
            }
        }
        other => write_bound(other, ctx, as_expr),
    }
}

fn write_bound(value: Value, ctx: &mut Context, as_expr: bool) -> Result<Value, Error> {
    if !as_expr {
        if let Value::Entry(e) = &value {
            ctx.set_write_dictionary(&e.table);
        }
        ctx.write(&value.to_print())?;
    }
    Ok(value)
}

fn eval_pattern(
    nodes: &[Node],
    spread: Option<Vec<(String, Value)>>,
    ctx: &mut Context,
    eval_sequence: EvalSeq,
    eval_expr: EvalExpr,
    as_expr: bool,
) -> Result<Value, Error> {
    if let Some(pairs) = spread {
        ctx.push_frame();
        for (k, v) in pairs {
            ctx.bind(k, v);
        }
        let result = if as_expr {
            eval_expr(nodes, ctx)
        } else {
            eval_sequence(nodes, ctx).map(|()| Value::Nil)
        };
        ctx.pop_frame();
        result
    } else if as_expr {
        eval_expr(nodes, ctx)
    } else {
        eval_sequence(nodes, ctx).map(|()| Value::Nil)
    }
}

fn run_map(
    tag: &TagNode,
    args: &[Vec<Node>],
    ctx: &mut Context,
    eval_expr: EvalExpr,
) -> Result<Value, Error> {
    if args.is_empty() {
        return Ok(Value::Map(Vec::new()));
    }
    let mut pairs = Vec::new();
    let mut i = 0usize;
    let first = eval_expr(&args[0], ctx)?;
    if let Some(base) = as_map(first.clone(), ctx) {
        pairs = base;
        i = 1;
    }
    let rest = &args[i..];
    if rest.len() % 2 != 0 {
        return Err(Error::runtime(
            "[map] needs key/value pairs",
            Some(tag.span),
        ));
    }
    if i == 0 {
        let key = first.to_print().trim().to_string();
        if key.is_empty() {
            return Err(Error::runtime("[map] key is empty", Some(tag.span)));
        }
        let value = bind_let_value(&args[1], ctx, eval_expr)?;
        map_put(&mut pairs, key, value);
        i = 2;
    }
    for chunk in args[i..].chunks(2) {
        let key = eval_expr(&chunk[0], ctx)?.to_print().trim().to_string();
        if key.is_empty() {
            return Err(Error::runtime("[map] key is empty", Some(tag.span)));
        }
        let value = bind_let_value(&chunk[1], ctx, eval_expr)?;
        map_put(&mut pairs, key, value);
    }
    Ok(Value::Map(pairs))
}

fn as_map(v: Value, ctx: &Context) -> Option<Vec<(String, Value)>> {
    match deref_value(v, ctx) {
        Value::Map(pairs) => Some(pairs),
        _ => None,
    }
}

fn map_put(pairs: &mut Vec<(String, Value)>, key: String, value: Value) {
    if let Some(slot) = pairs.iter_mut().find(|(k, _)| *k == key) {
        slot.1 = value;
    } else {
        pairs.push((key, value));
    }
}

fn map_get<'a>(pairs: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v)
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
        Value::Map(pairs) => pairs.into_iter().map(|(_, v)| v).collect(),
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
