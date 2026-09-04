use skald::{CaseMode, Options, RUN_PROFILE, Seed, skald};

fn run(pattern: &str, seed: Seed) -> String {
    skald(
        pattern,
        &Options {
            seed: Some(seed),
            case_mode: Some(CaseMode::None),
            ..Options::default()
        },
    )
    .unwrap()
}

#[test]
fn parse_canonical_integers() {
    assert_eq!(Seed::parse("0").unwrap(), Seed::Int(0));
    assert_eq!(Seed::parse("42").unwrap(), Seed::Int(42));
    assert_eq!(
        Seed::parse("9007199254740993").unwrap(),
        Seed::Int(9007199254740993)
    );
    assert_eq!(
        Seed::parse("18446744073709551615").unwrap(),
        Seed::Int(u64::MAX)
    );
}

#[test]
fn parse_rejects_ambiguous_numbers() {
    for s in [
        "", "042", "00", "+42", "-1", "1.5", "1e2", "1E+1", " 42", "42 ",
    ] {
        assert!(Seed::parse(s).is_err(), "{s:?}");
    }
    assert!(Seed::parse("18446744073709551616").is_err());
}

#[test]
fn parse_text_and_explicit_type() {
    assert_eq!(Seed::parse("hello").unwrap(), Seed::Text("hello".into()));
    assert_eq!(Seed::parse("hero-1").unwrap(), Seed::Text("hero-1".into()));
    assert_eq!(Seed::parse("text:42").unwrap(), Seed::Text("42".into()));
    assert_eq!(Seed::parse("text:042").unwrap(), Seed::Text("042".into()));
    assert!(Seed::parse("text:").is_err());
}

#[test]
fn integer_and_text_digit_seeds_differ() {
    let pattern = "{A|B|C|D|E|F|G|H}";
    let a = run(pattern, Seed::parse("42").unwrap());
    let b = run(pattern, Seed::parse("text:42").unwrap());
    assert_ne!(a, b);
}

#[test]
fn large_u64_is_stable() {
    let pattern = "{A|B|C|D}";
    let seed = Seed::parse("9007199254740993").unwrap();
    assert_eq!(run(pattern, seed.clone()), run(pattern, seed));
}

#[test]
fn run_profile_is_named() {
    assert_eq!(RUN_PROFILE, "skald-pcg32-v1");
}
