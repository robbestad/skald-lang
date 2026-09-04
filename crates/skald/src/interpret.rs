use crate::ast::{BlockNode, Node};
use crate::error::Error;
use crate::format::case::apply_case;
use crate::functions::run_tag;
use crate::output::{Choice, rewrite_part_texts};
use crate::query::resolve_query;
use crate::runtime::{BlockAttrs, Captured, Context, Rep, UserFn, capture, capture_ex};
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
            s.text
                .trim()
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
    let parent_emit = ctx.emit_target();
    let channel = attrs
        .out
        .clone()
        .or(parent_emit.1)
        .unwrap_or_else(|| ctx.channel.clone());
    let will_emit = parent_emit.0;
    let mut pieces: Vec<Captured> = Vec::new();
    for r in 0..times {
        ctx.rep_index = r;
        let idx = if let Some(name) = &attrs.sync {
            ctx.pick_synced(name, block.alternatives.len())
        } else {
            pick_weighted(block, ctx)?
        };
        if let Some(choices) = ctx.choices.as_mut() {
            choices.push(Choice {
                kind: "block".to_string(),
                span: block.span,
                alternative: idx,
                repeat_index: r,
                channel: if will_emit {
                    Some(channel.clone())
                } else {
                    None
                },
                emitted: will_emit,
            });
        }
        let piece = capture_ex(ctx, will_emit, Some(channel.clone()), |c| {
            if let Some(alt) = block.alternatives.get(idx) {
                eval_sequence(&alt.nodes, c)?;
            }
            Ok(())
        })?;
        pieces.push(piece);
    }
    ctx.rep_index = saved_index;
    let sep = if let Some(sep_nodes) = &attrs.sep {
        capture(ctx, |c| eval_sequence(sep_nodes, c))?
    } else {
        Captured::default()
    };
    let mut combined = Captured::default();
    for (i, piece) in pieces.into_iter().enumerate() {
        if i > 0 {
            if sep.parts.is_empty() {
                combined.push_glue(&sep.text);
            } else {
                combined.append(sep.clone());
            }
        }
        combined.append(piece);
    }
    let text = combined.text.clone();
    if let Some(name) = &attrs.out {
        ctx.emit_to_channel(name, combined)?;
    } else {
        ctx.emit_captured(combined)?;
    }
    Ok(Value::Str(text))
}

fn eval_node(node: &Node, ctx: &mut Context) -> Result<Value, Error> {
    ctx.tick(node.span())?;
    match node {
        Node::Text(t) => {
            ctx.set_write_glue();
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
            ctx.set_write_glue();
            ctx.write(&s)?;
            Ok(Value::Str(s))
        }
        Node::Query(q) => {
            let r = resolve_query(q, ctx)?;
            if let Some(entry) = &r.entry {
                ctx.set_write_dictionary(&entry.table);
            } else {
                ctx.set_write_glue();
            }
            ctx.write(&r.text)?;
            Ok(r.into_value())
        }
        Node::Block(b) => eval_block(b, ctx),
        Node::Tag(t) => {
            ctx.set_write_glue();
            run_tag(t, ctx, eval_sequence, eval_expr, false)
        }
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
                return Ok(finalize_expr(produced, printed.text));
            }
            Node::Query(q) => {
                ctx.capture_frames.push(crate::runtime::CaptureFrame {
                    text: String::new(),
                    parts: Vec::new(),
                    will_emit: false,
                    channel: None,
                });
                let r = resolve_query(q, ctx);
                ctx.capture_frames.pop();
                return Ok(r?.into_value());
            }
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
    Ok(finalize_expr(produced, printed.text))
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
                if !piece.text.is_empty() {
                    write_with_article(ctx, piece)?;
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
            write_with_article(ctx, piece)?;
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

fn write_with_article(ctx: &mut Context, piece: Captured) -> Result<(), Error> {
    let piece = piece.trim_start();
    if piece.text.is_empty() {
        return Ok(());
    }
    let mut out = Captured::default();
    let article = crate::format::article::with_article(&piece.text);
    let prefix = article
        .strip_suffix(piece.text.trim_start())
        .unwrap_or("a ")
        .to_string();
    out.push_glue(&prefix);
    out.append(piece);
    ctx.emit_captured(out)
}

pub fn interpret_output(nodes: &[Node], ctx: &mut Context) -> Result<crate::output::Output, Error> {
    eval_sequence(nodes, ctx)?;
    let mut channels = ctx.channels.clone();
    if !channels.contains_key("main") {
        channels.insert("main".to_string(), String::new());
    }
    let mut parts_by_channel = ctx.parts_by_channel.take().unwrap_or_default();
    for (name, value) in channels.iter_mut() {
        if name == "main" {
            *value = apply_case(value, ctx.case_mode);
            if let Some(parts) = parts_by_channel.get_mut("main") {
                rewrite_part_texts(parts, value);
            }
        }
    }
    let text = channels.get("main").cloned().unwrap_or_default();
    let picks = ctx.picks.take().unwrap_or_default();
    let parts = parts_by_channel.get("main").cloned().unwrap_or_default();
    let notes = std::mem::take(&mut ctx.notes);
    let unresolved = std::mem::take(&mut ctx.unresolved);
    let choices = ctx.choices.take().unwrap_or_default();
    let density = if parts.is_empty() {
        None
    } else {
        Some(crate::output::Density::from_parts(&parts))
    };
    Ok(crate::output::Output {
        text,
        channels,
        picks,
        parts,
        parts_by_channel,
        density,
        notes,
        choices,
        diagnostics: Vec::new(),
        unresolved,
    })
}
