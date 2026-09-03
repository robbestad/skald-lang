use skald::{CaseMode, Error, Options, Seed, explain, skald, skald_output};
use std::env;
use std::fs;
use std::process;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    eprintln!(
        "\
Usage: skald [options] <pattern>
       skald [options] -f <file>
       skald [options] -e <pattern>

Generate procedural text from a Skald pattern.

Options:
  -s, --seed <value>   Seed the generator (integer or string)
  -e, --eval <pattern> Pattern on the command line
  -f, --file <path>    Read the pattern from a file
      --case <mode>    none|first|word|title|upper|lower|sentence
      --nsfw           Include NSFW dictionary entries
      --channels       Print JSON with text and named channels
      --explain        Print JSON with text, channels, and dictionary picks
  -h, --help           Show this help
  -v, --version        Show version"
    );
}

fn parse_case(s: &str) -> Option<CaseMode> {
    match s.trim().to_ascii_lowercase().as_str() {
        "none" => Some(CaseMode::None),
        "default" | "first" => Some(CaseMode::First),
        "word" => Some(CaseMode::Word),
        "title" => Some(CaseMode::Title),
        "upper" => Some(CaseMode::Upper),
        "lower" => Some(CaseMode::Lower),
        "sentence" => Some(CaseMode::Sentence),
        _ => None,
    }
}

fn parse_seed(s: &str) -> Seed {
    if let Ok(n) = s.parse::<u64>() {
        Seed::Int(n)
    } else {
        Seed::Text(s.to_string())
    }
}

fn main() {
    match run(env::args().skip(1).collect()) {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    }
}

fn run(argv: Vec<String>) -> Result<i32, Error> {
    let mut seed = None;
    let mut file = None;
    let mut eval = None;
    let mut case_mode = None;
    let mut nsfw = false;
    let mut channels = false;
    let mut explain_run = false;
    let mut help = false;
    let mut version = false;
    let mut rest = Vec::new();
    let mut i = 0usize;
    while i < argv.len() {
        let arg = &argv[i];
        match arg.as_str() {
            "-h" | "--help" => help = true,
            "-v" | "--version" => version = true,
            "-s" | "--seed" => {
                i += 1;
                seed = argv.get(i).map(|s| parse_seed(s));
            }
            "-e" | "--eval" => {
                i += 1;
                eval = argv.get(i).cloned();
            }
            "-f" | "--file" => {
                i += 1;
                file = argv.get(i).cloned();
            }
            "--case" => {
                i += 1;
                let raw = argv.get(i).map(|s| s.as_str()).unwrap_or("");
                case_mode = parse_case(raw);
                if case_mode.is_none() {
                    return Err(Error::runtime(format!("Unknown case mode: {raw}"), None));
                }
            }
            "--nsfw" => nsfw = true,
            "--channels" => channels = true,
            "--explain" => explain_run = true,
            s if s.starts_with('-') => {
                return Err(Error::runtime(format!("Unknown option: {s}"), None));
            }
            _ => rest.push(arg.clone()),
        }
        i += 1;
    }

    if help {
        print_help();
        return Ok(0);
    }
    if version {
        println!("{VERSION}");
        return Ok(0);
    }

    let mut pattern = eval.or_else(|| {
        if rest.is_empty() {
            None
        } else {
            Some(rest.join(" "))
        }
    });
    if let Some(path) = file {
        pattern = Some(
            fs::read_to_string(&path).map_err(|e| Error::runtime(format!("{path}: {e}"), None))?,
        );
    }
    let Some(pattern) = pattern else {
        print_help();
        return Ok(1);
    };

    let opts = Options {
        seed,
        case_mode,
        nsfw,
        dictionary: None,
    };
    let out = if explain_run {
        explain(&pattern, &opts)?.to_json()
    } else if channels {
        skald_output(&pattern, &opts)?.to_json()
    } else {
        skald(&pattern, &opts)?
    };
    if out.ends_with('\n') {
        print!("{out}");
    } else {
        println!("{out}");
    }
    Ok(0)
}
