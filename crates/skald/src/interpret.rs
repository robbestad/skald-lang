use crate::ast::{BlockNode, Node};
use crate::error::Error;
use crate::format::article::with_article;
use crate::format::case::apply_case;
use crate::functions::run_tag;
use crate::query::resolve_query;
use crate::runtime::{BlockAttrs, Context, Rep, UserFn, capture};
use crate::value::Value;

const LETTERS: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];
const DIGITS: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

fn pick_weighted(block: &BlockNode, ctx: &mut Context) -> Result<usize, Error> {
    let n = block.alternatives.len();
    if n == 0 {
        return Ok(0);
    }
    let mut weights = Vec::with_capacity(n);
    for alt in &block.alternatives {
        let w = if let Some(weight) = &alt.weight {
            let s = capture(ctx, |c| eval_sequence(weight, c))?;
            s.trim()
                .parse::<f64>()
                .ok()
                .filter(|v| *v >= 0.0)
                .unwrap_or(1.0)
        } else {
            1.0
        };
        weights.push(w);
    }
    let sum: f64 = weights.iter().sum();
    if sum <= 0.0 {
        return Ok(ctx.rng.int(n as i64) as usize);
    }
    let mut r = ctx.rng.next() * sum;
    for (i, w) in weights.iter().enumerate() {
        r -= w;
        if r <= 0.0 {
            return Ok(i);
        }
    }
    Ok(n - 1)
}

fn eval_block(block: &BlockNode, ctx: &mut Context) -> Result<Value, Error> {
    let attrs = std::mem::take(&mut ctx.attrs);
    ctx.attrs = BlockAttrs::default();
    if let Some(def) = attrs.fn_def {
        ctx.functions.insert(
            def.name,
            UserFn {
                params: def.params,
                body: vec![Node::Block(block.clone())],
            },
        );
        return Ok(Value::Nil);
    }
    if attrs.chance < 100.0 && ctx.rng.next() * 100.0 >= attrs.chance {
        return Ok(Value::Nil);
    }
    let times = match attrs.rep {
        Rep::Each => block.alternatives.len().max(1) as i64,
        Rep::Times(n) => n,
    };
    let saved_index = ctx.rep_index;
    let mut parts = Vec::new();
    for r in 0..times {
        ctx.rep_index = r;
        let idx = if let Some(name) = &attrs.sync {
            ctx.pick_synced(name, block.alternatives.len())
        } else {
            pick_weighted(block, ctx)?
        };
        let piece = capture(ctx, |c| {
            if let Some(alt) = block.alternatives.get(idx) {
                eval_sequence(&alt.nodes, c)?;
            }
            Ok(())
        })?;
        parts.push(piece);
    }
    ctx.rep_index = saved_index;
    let sep = if let Some(sep_nodes) = &attrs.sep {
        capture(ctx, |c| eval_sequence(sep_nodes, c))?
    } else {
        String::new()
    };
    let text = parts.join(&sep);
    if let Some(name) = &attrs.out {
        ctx.write_channel(name, &text)?;
    } else {
        ctx.write(&text)?;
    }
    Ok(Value::Str(text))
}

fn eval_node(node: &Node, ctx: &mut Context) -> Result<Value, Error> {
    ctx.tick(node.span())?;
    match node {
        Node::Text(t) => {
            ctx.write(&t.value)?;
            Ok(Value::Str(t.value.clone()))
        }
        Node::Escape(e) => {
            let s = if e.code == "C" {
                ctx.rng.pick(LETTERS).copied().unwrap_or('A').to_string()
            } else if e.code == "d" {
                ctx.rng.pick(DIGITS).copied().unwrap_or('0').to_string()
            } else {
                e.code.clone()
            };
            ctx.write(&s)?;
            Ok(Value::Str(s))
        }
        Node::Query(q) => {
            let r = resolve_query(q, ctx)?;
            ctx.write(&r.text)?;
            Ok(r.into_value())
        }
        Node::Block(b) => eval_block(b, ctx),
        Node::Tag(t) => run_tag(t, ctx, eval_sequence, eval_expr, false),
    }
}

fn is_block(node: Option<&Node>) -> bool {
    matches!(node, Some(Node::Block(_)))
}

fn is_ws_text(node: &Node) -> bool {
    matches!(node, Node::Text(t) if t.value.chars().all(char::is_whitespace))
}

fn trim_expr_nodes(nodes: &[Node]) -> &[Node] {
    let start = nodes
        .iter()
        .position(|n| !is_ws_text(n))
        .unwrap_or(nodes.len());
    let end = nodes
        .iter()
        .rposition(|n| !is_ws_text(n))
        .map(|i| i + 1)
        .unwrap_or(0);
    if start >= end {
        &[]
    } else {
        &nodes[start..end]
    }
}

fn finalize_expr(produced: Value, printed: String) -> Value {
    let rendered = produced.to_print();
    if !printed.is_empty() && printed != rendered {
        return Value::Str(printed.trim().to_string());
    }
    match produced {
        Value::Nil if !printed.is_empty() => Value::Str(printed.trim().to_string()),
        Value::Str(s) if s.is_empty() && !printed.is_empty() => {
            Value::Str(printed.trim().to_string())
        }
        other => other,
    }
}

pub fn eval_expr(nodes: &[Node], ctx: &mut Context) -> Result<Value, Error> {
    let nodes = trim_expr_nodes(nodes);
    if nodes.is_empty() {
        return Ok(Value::Nil);
    }
    if nodes.len() == 1 {
        match &nodes[0] {
            Node::Tag(t) => {
                let mut produced = Value::Nil;
                let printed = capture(ctx, |c| {
                    produced = run_tag(t, c, eval_sequence, eval_expr, true)?;
                    Ok(())
                })?;
                return Ok(finalize_expr(produced, printed));
            }
            Node::Query(q) => return Ok(resolve_query(q, ctx)?.into_value()),
            Node::Text(t) => return Ok(Value::Str(t.value.trim().to_string())),
            Node::Escape(e) => {
                let s = if e.code == "C" {
                    ctx.rng.pick(LETTERS).copied().unwrap_or('A').to_string()
                } else if e.code == "d" {
                    ctx.rng.pick(DIGITS).copied().unwrap_or('0').to_string()
                } else {
                    e.code.clone()
                };
                return Ok(Value::Str(s));
            }
            Node::Block(_) => {}
        }
    }
    let mut produced = Value::Nil;
    let printed = capture(ctx, |c| {
        produced = eval_sequence_value(nodes, c)?;
        Ok(())
    })?;
    Ok(finalize_expr(produced, printed))
}

fn needs_capture(ctx: &Context) -> bool {
    ctx.pending_article
        || !matches!(ctx.attrs.rep, Rep::Times(1))
        || ctx.attrs.sep.is_some()
        || ctx.attrs.chance < 100.0
        || ctx.attrs.sync.is_some()
        || ctx.attrs.fn_def.is_some()
        || ctx.attrs.out.is_some()
}

fn eval_sequence_value(nodes: &[Node], ctx: &mut Context) -> Result<Value, Error> {
    let mut i = 0usize;
    let mut last = Value::Nil;
    while i < nodes.len() {
        let node = &nodes[i];

        if let Node::Tag(tag) = node {
            if ctx.pending_article {
                let piece = capture(ctx, |c| {
                    last = run_tag(tag, c, eval_sequence, eval_expr, false)?;
                    Ok(())
                })?;
                if !piece.is_empty() {
                    ctx.write(&with_article(&piece))?;
                    ctx.pending_article = false;
                }
            } else {
                last = run_tag(tag, ctx, eval_sequence, eval_expr, false)?;
            }
            i += 1;
            continue;
        }

        if needs_capture(ctx) {
            if let Node::Text(t) = node {
                if t.value.chars().all(char::is_whitespace) {
                    if !ctx.pending_article {
                        ctx.write(&t.value)?;
                    }
                    i += 1;
                    continue;
                }
            }
        }

        if let Some(carrier) = ctx.pending_if.take() {
            let mut taken: Vec<&Node> = Vec::new();
            let mut j = i;
            if is_block(Some(node)) {
                while j < nodes.len() && is_block(nodes.get(j)) && taken.len() < 2 {
                    taken.push(&nodes[j]);
                    j += 1;
                }
            } else {
                taken.push(node);
                j = i + 1;
            }
            let hit =
                ctx.match_carriers.contains_key(&carrier) || ctx.lookup_binding(&carrier).is_some();
            let chosen = if hit {
                taken.first().copied()
            } else {
                taken.get(1).copied()
            };
            if let Some(chosen) = chosen {
                last = eval_node(chosen, ctx)?;
            }
            i = j;
            continue;
        }

        if let Node::Block(b) = node {
            last = eval_block(b, ctx)?;
            i += 1;
            continue;
        }

        if ctx.pending_article {
            let piece = capture(ctx, |c| {
                last = eval_node(node, c)?;
                Ok(())
            })?;
            ctx.write(&with_article(&piece))?;
            ctx.pending_article = false;
        } else {
            last = eval_node(node, ctx)?;
        }
        i += 1;
    }
    Ok(last)
}

pub fn eval_sequence(nodes: &[Node], ctx: &mut Context) -> Result<(), Error> {
    eval_sequence_value(nodes, ctx).map(|_| ())
}

pub fn interpret_output(nodes: &[Node], ctx: &mut Context) -> Result<crate::output::Output, Error> {
    eval_sequence(nodes, ctx)?;
    let mut channels = ctx.channels.clone();
    if !channels.contains_key("main") {
        channels.insert("main".to_string(), String::new());
    }
    for value in channels.values_mut() {
        *value = apply_case(value, ctx.case_mode);
    }
    let text = channels.get("main").cloned().unwrap_or_default();
    let picks = ctx.picks.take().unwrap_or_default();
    Ok(crate::output::Output {
        text,
        channels,
        picks,
    })
}
