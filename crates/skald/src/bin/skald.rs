use skald::{
    Budget, CaseMode, Error, Options, Seed, explain, parse_pron_sidecar, skald, skald_output,
};
use std::env;
use std::fs;
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::process;
use std::sync::Arc;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    eprintln!(
        "\
Usage: skald [options] <pattern>
       skald [options] -f <file>
       skald [options] -e <pattern>
       skald                 (REPL, or read stdin if piped)

Generate procedural text from a Skald pattern.

Options:
  -s, --seed <value>   Seed the generator (integer or string)
  -e, --eval <pattern> Pattern on the command line
  -f, --file <path>    Read the pattern from a file
      --case <mode>    none|first|word|title|upper|lower|sentence
      --nsfw           Include NSFW dictionary entries
      --channels       Print JSON with text and named channels
      --explain        Print JSON with text, channels, and dictionary picks
      --prove          Like --explain, plus glue vs dictionary parts and density
      --story          Explain JSON plus story-lint notes (exit 2 if any story notes)
      --pron <path>    Extra pronunciations (word + X-SAMPA per line)
      --max-steps <n>  Step budget (default 100000)
      --max-output <n> Output-byte budget (default 1000000)
      --max-depth <n>  Call-depth budget (default 64)
  -h, --help           Show this help
  -v, --version        Show version

REPL commands: :seed, :case, :prove, :channels, :help, :quit"
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

struct Flags {
    seed: Option<Seed>,
    file: Option<String>,
    eval: Option<String>,
    case_mode: Option<CaseMode>,
    nsfw: bool,
    channels: bool,
    explain_run: bool,
    story: bool,
    help: bool,
    version: bool,
    pron: Option<String>,
    budget: Budget,
    rest: Vec<String>,
}

fn parse_flags(argv: &[String]) -> Result<Flags, Error> {
    let mut flags = Flags {
        seed: None,
        file: None,
        eval: None,
        case_mode: None,
        nsfw: false,
        channels: false,
        explain_run: false,
        story: false,
        help: false,
        version: false,
        pron: None,
        budget: Budget::default(),
        rest: Vec::new(),
    };
    let mut i = 0usize;
    while i < argv.len() {
        let arg = &argv[i];
        match arg.as_str() {
            "-h" | "--help" => flags.help = true,
            "-v" | "--version" => flags.version = true,
            "-s" | "--seed" => {
                i += 1;
                flags.seed = argv.get(i).map(|s| parse_seed(s));
            }
            "-e" | "--eval" => {
                i += 1;
                flags.eval = argv.get(i).cloned();
            }
            "-f" | "--file" => {
                i += 1;
                flags.file = argv.get(i).cloned();
            }
            "--case" => {
                i += 1;
                let raw = argv.get(i).map(|s| s.as_str()).unwrap_or("");
                flags.case_mode = parse_case(raw);
                if flags.case_mode.is_none() {
                    return Err(Error::runtime(format!("Unknown case mode: {raw}"), None));
                }
            }
            "--nsfw" => flags.nsfw = true,
            "--channels" => flags.channels = true,
            "--explain" | "--prove" => flags.explain_run = true,
            "--story" => {
                flags.story = true;
                flags.explain_run = true;
            }
            "--pron" => {
                i += 1;
                flags.pron = argv.get(i).cloned();
            }
            "--max-steps" => {
                i += 1;
                flags.budget.max_steps = parse_u32(argv.get(i), "max-steps")?;
            }
            "--max-output" => {
                i += 1;
                flags.budget.max_output = parse_usize(argv.get(i), "max-output")?;
            }
            "--max-depth" => {
                i += 1;
                flags.budget.max_depth = parse_u32(argv.get(i), "max-depth")?;
            }
            s if s.starts_with('-') => {
                return Err(Error::runtime(format!("Unknown option: {s}"), None));
            }
            _ => flags.rest.push(arg.clone()),
        }
        i += 1;
    }
    Ok(flags)
}

fn parse_u32(raw: Option<&String>, name: &str) -> Result<u32, Error> {
    let raw = raw.ok_or_else(|| Error::runtime(format!("--{name} needs a number"), None))?;
    raw.parse::<u32>()
        .map_err(|_| Error::runtime(format!("--{name} needs a number"), None))
}

fn parse_usize(raw: Option<&String>, name: &str) -> Result<usize, Error> {
    let raw = raw.ok_or_else(|| Error::runtime(format!("--{name} needs a number"), None))?;
    raw.parse::<usize>()
        .map_err(|_| Error::runtime(format!("--{name} needs a number"), None))
}

fn options_from(flags: &Flags) -> Result<Options, Error> {
    let pronunciations = match &flags.pron {
        Some(path) => {
            let src = fs::read_to_string(path)
                .map_err(|e| Error::runtime(format!("{path}: {e}"), None))?;
            Some(Arc::new(parse_pron_sidecar(&src)))
        }
        None => None,
    };
    Ok(Options {
        seed: flags.seed.clone(),
        case_mode: flags.case_mode,
        nsfw: flags.nsfw,
        dictionary: None,
        budget: flags.budget,
        pronunciations,
        story: flags.story,
        merge: false,
    })
}

fn render(pattern: &str, flags: &Flags) -> Result<String, Error> {
    let opts = options_from(flags)?;
    if flags.explain_run {
        Ok(explain(pattern, &opts)?.to_json())
    } else if flags.channels {
        Ok(skald_output(pattern, &opts)?.to_json())
    } else {
        skald(pattern, &opts)
    }
}

fn print_out(out: &str) {
    if out.ends_with('\n') {
        print!("{out}");
    } else {
        println!("{out}");
    }
}

fn run(argv: Vec<String>) -> Result<i32, Error> {
    let flags = parse_flags(&argv)?;

    if flags.help {
        print_help();
        return Ok(0);
    }
    if flags.version {
        println!("{VERSION}");
        return Ok(0);
    }

    let mut pattern = flags.eval.clone().or_else(|| {
        if flags.rest.is_empty() {
            None
        } else {
            Some(flags.rest.join(" "))
        }
    });
    if let Some(path) = &flags.file {
        pattern = Some(
            fs::read_to_string(path).map_err(|e| Error::runtime(format!("{path}: {e}"), None))?,
        );
    }
    if let Some(pattern) = pattern {
        if flags.story {
            let opts = options_from(&flags)?;
            let out = explain(&pattern, &opts)?;
            print_out(&out.to_json());
            let story_notes = out.notes.iter().any(|n| n.starts_with("story:"));
            return Ok(if story_notes { 2 } else { 0 });
        }
        print_out(&render(&pattern, &flags)?);
        return Ok(0);
    }

    if io::stdin().is_terminal() {
        return repl(flags);
    }

    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| Error::runtime(format!("stdin: {e}"), None))?;
    if buf.trim().is_empty() {
        print_help();
        return Ok(1);
    }
    print_out(&render(&buf, &flags)?);
    Ok(0)
}

fn repl(mut flags: Flags) -> Result<i32, Error> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    eprintln!("skald {VERSION} — type a pattern, or :help");
    loop {
        eprint!("skald> ");
        let _ = stdout.flush();
        let mut line = String::new();
        let n = stdin
            .lock()
            .read_line(&mut line)
            .map_err(|e| Error::runtime(format!("stdin: {e}"), None))?;
        if n == 0 {
            eprintln!();
            return Ok(0);
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(cmd) = line.strip_prefix(':') {
            match handle_repl_cmd(cmd.trim(), &mut flags) {
                ReplCmd::Continue => continue,
                ReplCmd::Quit => return Ok(0),
            }
        }
        if matches!(line, "exit" | "quit") {
            return Ok(0);
        }
        match render(line, &flags) {
            Ok(out) => print_out(&out),
            Err(err) => eprintln!("{err}"),
        }
    }
}

enum ReplCmd {
    Continue,
    Quit,
}

fn handle_repl_cmd(cmd: &str, flags: &mut Flags) -> ReplCmd {
    let mut parts = cmd.splitn(2, char::is_whitespace);
    let name = parts.next().unwrap_or("").to_ascii_lowercase();
    let arg = parts.next().unwrap_or("").trim();
    match name.as_str() {
        "q" | "quit" | "exit" => ReplCmd::Quit,
        "help" | "h" => {
            print_help();
            ReplCmd::Continue
        }
        "seed" => {
            if arg.is_empty() {
                flags.seed = None;
                eprintln!("seed: (none)");
            } else {
                flags.seed = Some(parse_seed(arg));
                eprintln!("seed: {arg}");
            }
            ReplCmd::Continue
        }
        "case" => {
            if arg.is_empty() {
                flags.case_mode = None;
                eprintln!("case: default");
            } else if let Some(mode) = parse_case(arg) {
                flags.case_mode = Some(mode);
                eprintln!("case: {arg}");
            } else {
                eprintln!("unknown case mode: {arg}");
            }
            ReplCmd::Continue
        }
        "prove" | "explain" => {
            flags.explain_run = !flags.explain_run;
            flags.channels = false;
            eprintln!("prove: {}", if flags.explain_run { "on" } else { "off" });
            ReplCmd::Continue
        }
        "channels" => {
            flags.channels = !flags.channels;
            flags.explain_run = false;
            eprintln!("channels: {}", if flags.channels { "on" } else { "off" });
            ReplCmd::Continue
        }
        _ => {
            eprintln!("unknown command :{cmd} — try :help");
            ReplCmd::Continue
        }
    }
}
