use skald::{CaseMode, Options, Seed, skald};

fn run(pattern: &str) -> String {
    skald(
        pattern,
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

fn err(pattern: &str) -> String {
    skald(
        pattern,
        &Options {
            seed: Some(Seed::Int(1)),
            case_mode: Some(CaseMode::None),
            ..Default::default()
        },
    )
    .unwrap_err()
    .to_string()
}

#[test]
fn define_and_call() {
    assert_eq!(
        run("[fn:greet; name]{Hello, [name]!}[greet: Ada]"),
        "Hello, Ada!"
    );
}

#[test]
fn definition_does_not_print() {
    assert_eq!(run("[fn:greet; name]{Hello}X"), "X");
}

#[test]
fn two_params() {
    assert_eq!(
        run("[fn:pair; left; right]{[left]+[right]}[pair: x; y]"),
        "x+y"
    );
}

#[test]
fn missing_arg_is_empty() {
    assert_eq!(run("[fn:greet; name]{([name])}[greet]"), "()");
}

#[test]
fn params_shadow_outer_lets() {
    assert_eq!(
        run("[let:name; outer][fn:f; name]{[name]}[f: inner] [name]"),
        "inner outer"
    );
}

#[test]
fn nested_calls() {
    assert_eq!(
        run("[fn:wrap; val]{([val])}[fn:greet; name]{[wrap: Hi [name]]}[greet: Ada]"),
        "(Hi Ada)"
    );
}

#[test]
fn choice_body_picks_per_call() {
    let out = run("[fn:coin]{A|A}[coin][coin]");
    assert_eq!(out, "AA");
}

#[test]
fn call_as_let_value() {
    assert_eq!(
        run("[fn:greet; name]{Hi [name]}[let:msg; [greet: Ada]][msg]!"),
        "Hi Ada!"
    );
}

#[test]
fn reserved_fn_name_errors() {
    let msg = err("[fn:let]{x}");
    assert!(msg.contains("reserved"), "{msg}");
    let msg = err("[fn:rhyme]{x}");
    assert!(msg.contains("reserved"), "{msg}");
    let msg = err("[fn:map]{x}");
    assert!(msg.contains("reserved"), "{msg}");
}

#[test]
fn unknown_call_still_errors() {
    let msg = err("[nope: 1]");
    assert!(msg.contains("Unknown tag"), "{msg}");
}

#[test]
fn recursion_hits_the_depth_budget() {
    let msg = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| err("[fn:bomb]{[bomb]}[bomb]"))
        .unwrap()
        .join()
        .expect("recursion thread");
    assert!(msg.contains("depth"), "{msg}");
}

#[test]
fn lets_inside_a_call_do_not_leak() {
    assert_eq!(
        run("[fn:f]{[let:tmp; in][tmp]}[f][if:tmp]{leaked}{ok}"),
        "inok"
    );
}
