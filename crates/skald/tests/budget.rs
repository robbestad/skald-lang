use skald::{Budget, CaseMode, Options, Seed, skald};

fn opts() -> Options {
    Options {
        seed: Some(Seed::Int(1)),
        case_mode: Some(CaseMode::None),
        ..Default::default()
    }
}

#[test]
fn tiny_step_budget_errors() {
    let err = skald(
        "[rep:50]{x}",
        &Options {
            budget: Budget {
                max_steps: 3,
                ..Budget::default()
            },
            ..opts()
        },
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("budget"), "{err}");
    assert!(err.contains("steps"), "{err}");
}

#[test]
fn tiny_output_budget_errors() {
    let err = skald(
        "[rep:20]{hello}",
        &Options {
            budget: Budget {
                max_output: 8,
                ..Budget::default()
            },
            ..opts()
        },
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("budget"), "{err}");
    assert!(err.contains("output"), "{err}");
}

#[test]
fn tiny_depth_budget_errors() {
    let err = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            skald(
                "[fn:bomb]{[bomb]}[bomb]",
                &Options {
                    budget: Budget {
                        max_depth: 4,
                        ..Budget::default()
                    },
                    ..opts()
                },
            )
            .unwrap_err()
            .to_string()
        })
        .unwrap()
        .join()
        .expect("depth thread");
    assert!(err.contains("depth"), "{err}");
}

#[test]
fn default_budget_still_runs_a_small_pattern() {
    assert_eq!(skald("[rep:3]{x}", &opts()).unwrap(), "xxx");
}
