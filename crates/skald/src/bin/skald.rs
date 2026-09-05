use skald::{
    Budget, CaseMode, Dictionary, Error, Options, Seed, explain, from_json, from_language_pack,
    parse_pron_sidecar, skald, skald_output,
};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::path::Path;
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

Artifact commands (sidecar is <file>.json next to the .skald):
  skald manifest <file.skald>   Write/update the sidecar from the pattern
  skald inspect <file.skald>    Show the sidecar without running a model
  skald verify <file.skald>     Check pattern hash and run profile
  skald run <file.skald>        Verify, then render (optional --seed writes a unique receipt)

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
      --dict <path>    Overlay dictionary JSON (repeatable; left to right)
      --dict-only      Ignore bundled English; use only --dict files
      --locale <id>    en-US (default), nb-NO, or nn-NO
      --pack <path>    Language pack JSON (required for nb-NO / nn-NO)
      --pron <path>    Extra pronunciations (word + X-SAMPA per line)
      --max-steps <n>  Step budget (default 100000)
      --max-output <n> Output-byte budget (default 1000000)
      --max-depth <n>  Call-depth budget (default 64)
  -h, --help           Show this help
  -v, --version        Show version

REPL commands: :seed, :case, :prove, :story, :channels, :help, :quit"
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

fn parse_seed(s: &str) -> Result<Seed, Error> {
    Seed::parse(s).map_err(|message| Error::runtime(message, None))
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
    nsfw_set: bool,
    channels: bool,
    explain_run: bool,
    story: bool,
    story_set: bool,
    help: bool,
    version: bool,
    pron: Option<String>,
    dicts: Vec<String>,
    dict_only: bool,
    locale: Option<String>,
    pack: Option<String>,
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
        nsfw_set: false,
        channels: false,
        explain_run: false,
        story: false,
        story_set: false,
        help: false,
        version: false,
        pron: None,
        dicts: Vec::new(),
        dict_only: false,
        locale: None,
        pack: None,
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
                let raw = argv
                    .get(i)
                    .ok_or_else(|| Error::runtime("missing --seed value", None))?;
                flags.seed = Some(parse_seed(raw)?);
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
            "--nsfw" => {
                flags.nsfw = true;
                flags.nsfw_set = true;
            }
            "--channels" => flags.channels = true,
            "--explain" | "--prove" => flags.explain_run = true,
            "--story" => {
                flags.story = true;
                flags.story_set = true;
                flags.explain_run = true;
            }
            "--pron" => {
                i += 1;
                flags.pron = argv.get(i).cloned();
            }
            "--dict" => {
                i += 1;
                let path = argv
                    .get(i)
                    .cloned()
                    .ok_or_else(|| Error::runtime("--dict needs a path", None))?;
                flags.dicts.push(path);
            }
            "--dict-only" => flags.dict_only = true,
            "--locale" => {
                i += 1;
                flags.locale = argv.get(i).cloned();
            }
            "--pack" => {
                i += 1;
                flags.pack = Some(
                    argv.get(i)
                        .cloned()
                        .ok_or_else(|| Error::runtime("--pack needs a path", None))?,
                );
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
    let loaded = load_language(flags)?;
    Ok(Options {
        seed: flags.seed.clone(),
        case_mode: flags.case_mode,
        nsfw: flags.nsfw,
        dictionary: loaded.dictionary,
        budget: flags.budget,
        pronunciations,
        story: flags.story,
        merge: loaded.merge,
        capabilities: loaded.capabilities,
        locale: loaded.locale,
        reject_unresolved: false,
    })
}

struct LoadedLanguage {
    dictionary: Option<Arc<Dictionary>>,
    capabilities: Option<skald::Capabilities>,
    locale: Option<String>,
    merge: bool,
}

fn load_language(flags: &Flags) -> Result<LoadedLanguage, Error> {
    let requested = flags.locale.clone();
    if let Some(path) = &flags.pack {
        let src =
            fs::read_to_string(path).map_err(|e| Error::runtime(format!("{path}: {e}"), None))?;
        let pack = from_language_pack(&src)?;
        if let Some(locale) = &requested {
            if locale != &pack.locale {
                return Err(Error::runtime(
                    format!(
                        "language pack locale {} does not match {locale}",
                        pack.locale
                    ),
                    None,
                ));
            }
        }
        let mut dict = pack.dictionary;
        for overlay in &flags.dicts {
            let extra_src = fs::read_to_string(overlay)
                .map_err(|e| Error::runtime(format!("{overlay}: {e}"), None))?;
            dict.overlay_keep_forms(&from_json(&extra_src)?)?;
        }
        return Ok(LoadedLanguage {
            dictionary: Some(Arc::new(dict)),
            capabilities: Some(pack.capabilities),
            locale: Some(pack.locale),
            merge: false,
        });
    }
    if requested.as_deref().is_some_and(|l| l != "en-US") {
        return Err(Error::runtime(
            format!("missing language pack for {}", requested.unwrap()),
            None,
        ));
    }
    Ok(LoadedLanguage {
        dictionary: load_dicts(&flags.dicts, flags.dict_only)?,
        capabilities: None,
        locale: requested.or_else(|| Some("en-US".into())),
        merge: false,
    })
}

fn load_dicts(paths: &[String], dict_only: bool) -> Result<Option<Arc<Dictionary>>, Error> {
    if paths.is_empty() && !dict_only {
        return Ok(None);
    }
    let mut base = if dict_only {
        Dictionary::empty()
    } else {
        (*skald::en_us()).clone()
    };
    for path in paths {
        let src =
            fs::read_to_string(path).map_err(|e| Error::runtime(format!("{path}: {e}"), None))?;
        let extra = from_json(&src)?;
        base.overlay(&extra);
    }
    Ok(Some(Arc::new(base)))
}

fn story_exit(out: &skald::Output) -> i32 {
    let story_err = out.diagnostics.iter().any(|d| d.severity == "error")
        || out.notes.iter().any(|n| n.starts_with("story:"));
    if story_err { 2 } else { 0 }
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
    let mut flags = parse_flags(&argv)?;

    if flags.help {
        print_help();
        return Ok(0);
    }
    if flags.version {
        println!("{VERSION}");
        return Ok(0);
    }

    if let Some(code) = artifact_command(&mut flags)? {
        return Ok(code);
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
        return run_pattern(&pattern, &flags);
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
    run_pattern(&buf, &flags)
}

fn apply_manifest_run_options(
    flags: &mut Flags,
    manifest: &skald::artifact::Manifest,
) -> Result<(), Error> {
    if flags.seed.is_none() {
        if let Some(seed) = &manifest.seed {
            let encoded = if seed.kind == "text" {
                format!("text:{}", seed.value)
            } else {
                seed.value.clone()
            };
            flags.seed = Some(parse_seed(&encoded)?);
        }
    }
    if flags.case_mode.is_none() {
        if let Some(case) = &manifest.case_mode {
            flags.case_mode = parse_case(case);
            if flags.case_mode.is_none() {
                return Err(Error::runtime(
                    format!("unknown case in manifest: {case}"),
                    None,
                ));
            }
        }
    }
    if !flags.nsfw_set {
        flags.nsfw = manifest.nsfw;
    }
    if !flags.story_set {
        flags.story = manifest.story;
        if flags.story {
            flags.explain_run = true;
        }
    }
    Ok(())
}

fn dict_dependencies(
    flags: &Flags,
    artifact_path: &Path,
) -> Result<Vec<skald::artifact::ManifestDependency>, Error> {
    let mut out = Vec::new();
    let mut push = |path: &str, role: &str| -> Result<(), Error> {
        let bytes = fs::read(path).map_err(|e| Error::runtime(format!("{path}: {e}"), None))?;
        out.push(skald::artifact::ManifestDependency {
            path: skald::artifact::stored_dependency_path(artifact_path, path)?,
            hash: skald::artifact::file_hash(&bytes),
            role: Some(role.to_string()),
        });
        Ok(())
    };
    if let Some(pack) = &flags.pack {
        push(pack, "pack")?;
    }
    for path in &flags.dicts {
        push(path, "dict")?;
    }
    if let Some(pron) = &flags.pron {
        push(pron, "pron")?;
    }
    Ok(out)
}

fn reject_locked_overrides(
    flags: &Flags,
    manifest: &skald::artifact::Manifest,
    cli_case: bool,
) -> Result<(), Error> {
    if !manifest.replay_locked() {
        return Ok(());
    }
    let case_changed = cli_case
        && case_recipe_key(flags.case_mode) != manifest_case_key(manifest.case_mode.as_deref());
    let nsfw_changed = flags.nsfw_set && flags.nsfw != manifest.nsfw;
    let story_changed = flags.story_set && flags.story != manifest.story;
    if case_changed || nsfw_changed || story_changed {
        return Err(Error::runtime(
            "locked artifact run rejects recipe overrides; pass --seed for a new instance or update the manifest",
            None,
        ));
    }
    Ok(())
}

fn looks_like_language_pack(src: &str) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(src) else {
        return false;
    };
    v.get("formatVersion").is_some() && v.get("locale").is_some() && v.get("capabilities").is_some()
}

fn apply_manifest_language(
    flags: &mut Flags,
    manifest: &skald::artifact::Manifest,
    artifact_path: &Path,
) -> Result<(), Error> {
    if flags.locale.is_none() && !manifest.locale.is_empty() {
        flags.locale = Some(manifest.locale.clone());
    }
    if manifest.dict_only {
        flags.dict_only = true;
    }
    let base = artifact_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if flags.pron.is_none() {
        for dep in &manifest.dependencies {
            if dep.role.as_deref() == Some("pron") {
                let resolved = skald::artifact::resolve_dependency_path(base, &dep.path);
                flags.pron = Some(resolved.to_string_lossy().into_owned());
            }
        }
    }
    if flags.pack.is_some() || !flags.dicts.is_empty() {
        return Ok(());
    }
    let mut pack = None;
    let mut overlays = Vec::new();
    for dep in &manifest.dependencies {
        if dep.role.as_deref() == Some("pron") {
            continue;
        }
        let resolved = skald::artifact::resolve_dependency_path(base, &dep.path);
        let src = fs::read_to_string(&resolved)
            .map_err(|e| Error::runtime(format!("{}: {e}", resolved.display()), None))?;
        let path_str = resolved.to_string_lossy().into_owned();
        if looks_like_language_pack(&src) && pack.is_none() {
            pack = Some(path_str);
        } else {
            overlays.push(path_str);
        }
    }
    flags.pack = pack;
    flags.dicts.extend(overlays);
    Ok(())
}

fn seed_from_flags(flags: &Flags) -> Option<skald::artifact::ManifestSeed> {
    flags
        .seed
        .as_ref()
        .map(skald::artifact::ManifestSeed::from_seed)
}

fn apply_receipt_seed(flags: &mut Flags, receipt: &skald::artifact::Receipt) -> Result<(), Error> {
    if let Some(seed) = &receipt.seed {
        flags.seed = Some(parse_seed(&seed.encode())?);
    }
    Ok(())
}

fn artifact_options(flags: &Flags) -> Result<Options, Error> {
    let mut opts = options_from(flags)?;
    opts.reject_unresolved = true;
    Ok(opts)
}

fn receipt_channels(out: &skald::Output) -> BTreeMap<String, String> {
    out.channels
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn case_recipe_key(mode: Option<CaseMode>) -> &'static str {
    match mode {
        None | Some(CaseMode::Default) | Some(CaseMode::First) => "first",
        Some(CaseMode::None) => "none",
        Some(CaseMode::Word) => "word",
        Some(CaseMode::Title) => "title",
        Some(CaseMode::Upper) => "upper",
        Some(CaseMode::Lower) => "lower",
        Some(CaseMode::Sentence) => "sentence",
    }
}

fn manifest_case_key(label: Option<&str>) -> &str {
    match label {
        None | Some("default") | Some("first") => "first",
        Some(other) => other,
    }
}

fn artifact_base_dir(path: &str) -> &Path {
    Path::new(path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn artifact_command(flags: &mut Flags) -> Result<Option<i32>, Error> {
    let Some(cmd) = flags.rest.first().map(|s| s.as_str()) else {
        return Ok(None);
    };
    if !matches!(cmd, "run" | "inspect" | "verify" | "manifest") {
        return Ok(None);
    }
    let path = flags.file.clone().or_else(|| flags.rest.get(1).cloned());
    let is_artifact =
        flags.file.is_some() || path.as_deref().is_some_and(|p| p.ends_with(".skald"));
    if !is_artifact {
        return Ok(None);
    }
    let cmd = flags.rest.remove(0);
    let path =
        path.ok_or_else(|| Error::runtime(format!("skald {cmd} needs a .skald file"), None))?;
    let pattern =
        fs::read_to_string(&path).map_err(|e| Error::runtime(format!("{path}: {e}"), None))?;
    let side = skald::artifact::sidecar_path(std::path::Path::new(&path));
    match cmd.as_str() {
        "manifest" => {
            let case = match flags.case_mode {
                Some(CaseMode::None) => Some("none"),
                Some(CaseMode::First) => Some("first"),
                Some(CaseMode::Word) => Some("word"),
                Some(CaseMode::Title) => Some("title"),
                Some(CaseMode::Upper) => Some("upper"),
                Some(CaseMode::Lower) => Some("lower"),
                Some(CaseMode::Sentence) => Some("sentence"),
                Some(CaseMode::Default) | None => None,
            };
            let opts = options_from(flags)?;
            let dict = opts.dictionary.clone().unwrap_or_else(skald::en_us);
            let dependencies = dict_dependencies(flags, Path::new(&path))?;
            let mut manifest = skald::artifact::Manifest::for_pattern_locked(
                &pattern,
                flags.seed.as_ref(),
                case,
                flags.nsfw,
                flags.story,
                opts.locale.as_deref().unwrap_or("en-US"),
                Some(dict.as_ref()),
                &dependencies,
            );
            manifest.dict_only = flags.dict_only;
            skald::artifact::write_manifest(&side, &manifest)?;
            println!("{}", side.display());
            Ok(Some(0))
        }
        "inspect" => {
            let manifest = skald::artifact::read_manifest(&side)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&manifest)
                    .map_err(|e| Error::runtime(e.to_string(), None))?
            );
            Ok(Some(0))
        }
        "verify" => {
            let manifest = skald::artifact::read_manifest(&side)?;
            skald::artifact::verify_pattern(&pattern, &manifest)?;
            let cli_seed = flags.seed.is_some();
            let cli_case = flags.case_mode.is_some();
            apply_manifest_run_options(flags, &manifest)?;
            apply_manifest_language(flags, &manifest, Path::new(&path))?;
            reject_locked_overrides(flags, &manifest, cli_case)?;
            let opts = artifact_options(flags)?;
            let dict = opts.dictionary.clone().unwrap_or_else(skald::en_us);
            skald::preflight_errors(
                &pattern,
                dict.as_ref(),
                opts.capabilities.as_ref(),
                flags.nsfw,
            )?;
            skald::artifact::verify_lock(&manifest, dict.as_ref(), artifact_base_dir(&path))?;
            let receipt_path = if cli_seed {
                skald::artifact::receipt_path_for(
                    Path::new(&path),
                    seed_from_flags(flags).as_ref(),
                    manifest.seed.as_ref(),
                )
            } else {
                skald::artifact::receipt_path(Path::new(&path))
            };
            let mut replayed = false;
            let mut legacy_receipt = false;
            if receipt_path.exists() {
                let receipt = skald::artifact::read_receipt(&receipt_path)?;
                apply_receipt_seed(flags, &receipt)?;
                let out = skald_output(&pattern, &artifact_options(flags)?)?;
                match skald::artifact::verify_receipt(
                    &receipt,
                    &out.text,
                    &receipt_channels(&out),
                    &pattern,
                )? {
                    skald::artifact::ReceiptReplay::Full => replayed = true,
                    skald::artifact::ReceiptReplay::LegacySkipped => legacy_receipt = true,
                }
            }
            if replayed {
                println!("ok {} receipt", manifest.pattern_hash);
            } else if legacy_receipt {
                println!(
                    "ok {} (legacy receipt; recipe verified, receipt not replayed)",
                    manifest.pattern_hash
                );
            } else if manifest.replay_locked() {
                println!("ok {} (recipe; no receipt)", manifest.pattern_hash);
            } else {
                println!(
                    "ok {} (formatVersion {}; replay not locked)",
                    manifest.pattern_hash, manifest.format_version
                );
            }
            Ok(Some(0))
        }
        "run" => {
            let manifest = skald::artifact::read_manifest(&side)?;
            skald::artifact::verify_pattern(&pattern, &manifest)?;
            let cli_seed = flags.seed.is_some();
            let cli_case = flags.case_mode.is_some();
            apply_manifest_run_options(flags, &manifest)?;
            apply_manifest_language(flags, &manifest, Path::new(&path))?;
            reject_locked_overrides(flags, &manifest, cli_case)?;
            if flags.seed.is_none() {
                flags.seed = Some(skald::artifact::choose_run_seed());
            }
            let opts = artifact_options(flags)?;
            let dict = opts.dictionary.clone().unwrap_or_else(skald::en_us);
            skald::preflight_errors(
                &pattern,
                dict.as_ref(),
                opts.capabilities.as_ref(),
                flags.nsfw,
            )?;
            skald::artifact::verify_lock(&manifest, dict.as_ref(), artifact_base_dir(&path))?;
            let out = skald_output(&pattern, &opts)?;
            let channels = receipt_channels(&out);
            let (display, code) = if flags.story {
                let explained = explain(&pattern, &opts)?;
                (explained.to_json(), story_exit(&explained))
            } else {
                (render(&pattern, flags)?, 0)
            };
            let receipt = skald::artifact::Receipt {
                format_version: skald::artifact::RECEIPT_FORMAT_VERSION,
                pattern_hash: manifest.pattern_hash.clone(),
                run_profile: manifest.run_profile.clone(),
                text: out.text,
                channels,
                seed: seed_from_flags(flags),
            };
            let rec_path = if cli_seed {
                skald::artifact::receipt_path_for(
                    Path::new(&path),
                    receipt.seed.as_ref(),
                    manifest.seed.as_ref(),
                )
            } else {
                skald::artifact::receipt_path(Path::new(&path))
            };
            skald::artifact::write_receipt(&rec_path, &receipt)?;
            print_out(&display);
            Ok(Some(code))
        }
        _ => Ok(None),
    }
}

fn run_pattern(pattern: &str, flags: &Flags) -> Result<i32, Error> {
    if flags.story {
        let opts = options_from(flags)?;
        let out = explain(pattern, &opts)?;
        print_out(&out.to_json());
        return Ok(story_exit(&out));
    }
    print_out(&render(pattern, flags)?);
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
                match parse_seed(arg) {
                    Ok(seed) => {
                        flags.seed = Some(seed);
                        eprintln!("seed: {arg}");
                    }
                    Err(err) => eprintln!("{err}"),
                }
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
        "story" => {
            flags.story = !flags.story;
            if flags.story {
                flags.explain_run = true;
                flags.channels = false;
            }
            eprintln!("story: {}", if flags.story { "on" } else { "off" });
            ReplCmd::Continue
        }
        _ => {
            eprintln!("unknown command :{cmd} — try :help");
            ReplCmd::Continue
        }
    }
}
