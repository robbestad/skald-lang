use skald::{CarrierKind, Node, TokenKind, parse, tokenize};

#[test]
fn tokenizes_queries_tags_braces_and_pipes() {
    let kinds: Vec<_> = tokenize("{a|b}<noun>")
        .into_iter()
        .map(|t| t.kind)
        .collect();
    assert!(matches!(
        kinds.as_slice(),
        [
            TokenKind::LBrace,
            TokenKind::Text(_),
            TokenKind::Pipe,
            TokenKind::Text(_),
            TokenKind::RBrace,
            TokenKind::Query(_),
            TokenKind::Eof
        ]
    ));
}

#[test]
fn treats_backslash_c_as_escape() {
    let tokens = tokenize("\\C");
    assert!(matches!(tokens[0].kind, TokenKind::Escape(ref c) if c == "C"));
}

#[test]
fn parses_plain_text() {
    let nodes = parse("Hello world").unwrap();
    assert!(matches!(nodes[0], Node::Text(ref t) if t.value == "Hello world"));
}

#[test]
fn parses_query_with_filter_sub_and_carrier() {
    let nodes = parse("<firstname male :: hero>").unwrap();
    match &nodes[0] {
        Node::Query(q) => {
            assert_eq!(q.table, "firstname");
            assert_eq!(q.args, vec!["male"]);
            assert!(q.exclude.is_empty());
            assert_eq!(q.carrier.as_deref(), Some("hero"));
            assert_eq!(q.carrier_kind, Some(CarrierKind::Match));
            assert_eq!(q.raw, "firstname male :: hero");
        }
        other => panic!("expected query, got {other:?}"),
    }
}

#[test]
fn parses_dotted_and_dashed_queries() {
    match &parse("<noun.plural>").unwrap()[0] {
        Node::Query(q) => {
            assert_eq!(q.table, "noun");
            assert_eq!(q.args, vec!["plural"]);
        }
        _ => panic!("expected query"),
    }
    match &parse("<verb-transitive>").unwrap()[0] {
        Node::Query(q) => {
            assert_eq!(q.table, "verb");
            assert_eq!(q.args, vec!["transitive"]);
        }
        _ => panic!("expected query"),
    }
    match &parse("<pron.poss-male>").unwrap()[0] {
        Node::Query(q) => {
            assert_eq!(q.table, "pron");
            assert_eq!(q.args, vec!["poss", "male"]);
        }
        _ => panic!("expected query"),
    }
}

#[test]
fn parses_recall_only_carrier() {
    match &parse("<::hero>").unwrap()[0] {
        Node::Query(q) => {
            assert_eq!(q.table, "");
            assert_eq!(q.carrier.as_deref(), Some("hero"));
        }
        _ => panic!("expected query"),
    }
}

#[test]
fn parses_form_args_after_a_carrier() {
    match &parse("<::pet plural>").unwrap()[0] {
        Node::Query(q) => {
            assert_eq!(q.carrier.as_deref(), Some("pet"));
            assert_eq!(q.args, vec!["plural"]);
        }
        _ => panic!("expected query"),
    }
}

#[test]
fn parses_rhyme_carrier_and_regex_constraint() {
    match &parse("<noun ::~a>").unwrap()[0] {
        Node::Query(q) => {
            assert_eq!(q.carrier_kind, Some(CarrierKind::Rhyme));
            assert_eq!(q.carrier.as_deref(), Some("a"));
        }
        _ => panic!("expected query"),
    }
    match &parse("<noun ::&a>").unwrap()[0] {
        Node::Query(q) => {
            assert_eq!(q.carrier_kind, Some(CarrierKind::Rhyme));
            assert_eq!(q.carrier.as_deref(), Some("a"));
        }
        _ => panic!("expected query"),
    }
    match &parse("<firstname ~ /^[AEIOU]/>").unwrap()[0] {
        Node::Query(q) => {
            assert_eq!(q.table, "firstname");
            assert_eq!(q.regex.as_deref(), Some("^[AEIOU]"));
            assert!(!q.regex_neg);
        }
        _ => panic!("expected query"),
    }
    match &parse("<noun-animal !~ /cat|dog/>").unwrap()[0] {
        Node::Query(q) => {
            assert_eq!(q.table, "noun");
            assert_eq!(q.args, vec!["animal"]);
            assert_eq!(q.regex.as_deref(), Some("cat|dog"));
            assert!(q.regex_neg);
        }
        _ => panic!("expected query"),
    }
}

#[test]
fn parses_plural_subtype() {
    match &parse("<noun..pl>").unwrap()[0] {
        Node::Query(q) => {
            assert_eq!(q.table, "noun");
            assert_eq!(q.plural_sub.as_deref(), Some("plural"));
            assert!(q.args.is_empty());
        }
        _ => panic!("expected query"),
    }
    match &parse("<noun.sg..pl>").unwrap()[0] {
        Node::Query(q) => {
            assert_eq!(q.args, vec!["sg"]);
            assert_eq!(q.plural_sub.as_deref(), Some("plural"));
        }
        _ => panic!("expected query"),
    }
}

#[test]
fn parses_negative_class_filters() {
    match &parse("<noun ?!animal>").unwrap()[0] {
        Node::Query(q) => assert_eq!(q.exclude, vec!["animal"]),
        _ => panic!("expected query"),
    }
    match &parse("<noun-!animal>").unwrap()[0] {
        Node::Query(q) => {
            assert_eq!(q.table, "noun");
            assert_eq!(q.exclude, vec!["animal"]);
        }
        _ => panic!("expected query"),
    }
}

#[test]
fn parses_block_with_alternatives() {
    let nodes = parse("{<noun>|<adj>|<verb>}").unwrap();
    match &nodes[0] {
        Node::Block(b) => {
            assert_eq!(b.alternatives.len(), 3);
            assert!(matches!(b.alternatives[0].nodes[0], Node::Query(_)));
        }
        _ => panic!("expected block"),
    }
}

#[test]
fn parses_plain_text_in_a_block() {
    let nodes = parse("{Example text}").unwrap();
    match &nodes[0] {
        Node::Block(b) => {
            assert!(b.alternatives[0].weight.is_none());
            assert!(
                matches!(b.alternatives[0].nodes[0], Node::Text(ref t) if t.value == "Example text")
            );
        }
        _ => panic!("expected block"),
    }
}

#[test]
fn parses_nested_blocks() {
    let nodes = parse("{a {b|c} d}").unwrap();
    match &nodes[0] {
        Node::Block(b) => {
            assert!(
                b.alternatives[0]
                    .nodes
                    .iter()
                    .any(|n| matches!(n, Node::Block(_)))
            );
        }
        _ => panic!("expected block"),
    }
}

#[test]
fn parses_tags_with_separator_escapes() {
    let nodes = parse("[rep:3][sep:\\n][case:title]").unwrap();
    match &nodes[0] {
        Node::Tag(t) => {
            assert_eq!(t.name, "rep");
            assert_eq!(t.arg, "3");
        }
        _ => panic!("expected tag"),
    }
    match &nodes[1] {
        Node::Tag(t) => {
            assert_eq!(t.name, "sep");
            assert_eq!(t.arg, "\n");
        }
        _ => panic!("expected tag"),
    }
    match &nodes[2] {
        Node::Tag(t) => {
            assert_eq!(t.name, "case");
            assert_eq!(t.arg, "title");
        }
        _ => panic!("expected tag"),
    }
}

#[test]
fn parse_errors_have_spans() {
    let err = parse("{unclosed").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("parse error"), "{msg}");
}
