use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, exit};
use std::sync::OnceLock;

use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper, history::DefaultHistory};
use serde::Deserialize;
use dirs::{cache_dir, config_dir};

const VERSION: &str = "3.5.0";
const PARU:    &str  = "paru";

// ── ANSI helpers ────────────────────────────────────────────────────────────

macro_rules! ansi {
    ($code:literal, $fn:ident) => {
        fn $fn(s: &str) -> String { format!(concat!("\x1b[", $code, "m{}\x1b[0m"), s) }
    };
}
ansi!("36", cyan);
ansi!("1",  bold);
ansi!("2",  dim);
ansi!("31", red);
ansi!("33", yellow);

// ── Config ───────────────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
struct CommandEntry {
    key:     String,
    action:  String,
    prompt:  Option<String>,
    desc:    Option<String>,
    confirm: Option<bool>,
}

#[derive(Deserialize)]
struct Config {
    commands: HashMap<String, CommandEntry>,
}

fn default_config_toml() -> &'static str {
    r#"[commands]
update  = { key = "u", action = "paru -Syu",                                   desc = "upgrade all packages"      }
install = { key = "i", action = "paru -S {pkg}",                               desc = "install a package",        prompt = "pkg"   }
remove  = { key = "r", action = "paru -R {pkg}",                               desc = "remove a package",         prompt = "pkg"   }
purge   = { key = "p", action = "paru -Rns {pkg}",                             desc = "remove package + deps",    prompt = "pkg",  confirm = true }
search  = { key = "s", action = "paru -Ss {query}",                            desc = "search packages",          prompt = "query" }
clean   = { key = "c", action = "paru -Sc",                                    desc = "clean package cache"       }
orphans = { key = "o", action = "shell:paru -Rns $(pacman -Qtdq)",             desc = "remove orphaned packages", confirm = true   }
log     = { key = "l", action = "shell:tail -n 50 /var/log/pacman.log | less", desc = "view recent pacman log"    }
quit    = { key = "q", action = "builtin:quit",                                desc = "exit archie"               }
help    = { key = "h", action = "builtin:help",                                desc = "show this help"            }
"#
}

fn config_path() -> PathBuf {
    config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("archie")
        .join("archie.toml")
}

fn validate_entry(entry: &CommandEntry) -> Result<(), String> {
    if entry.key.is_empty()    { return Err("key is empty".into()); }
    if entry.action.is_empty() { return Err("action is empty".into()); }
    if let Some(p) = &entry.prompt {
        if !entry.action.contains(&format!("{{{p}}}")) {
            return Err(format!("action missing placeholder {{{p}}}"));
        }
    }
    Ok(())
}

fn write_config_atomically(path: &Path, content: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() { fs::create_dir_all(dir)?; }
    let tmp = path.with_extension("toml.tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)
}

fn load_config() -> Config {
    let path = config_path();
    let mut commands = toml::from_str::<Config>(default_config_toml())
        .expect("default config is valid")
        .commands;

    if let Ok(content) = fs::read_to_string(&path) {
        match toml::from_str::<Config>(&content) {
            Ok(user_cfg) => {
                for (name, entry) in user_cfg.commands {
                    match validate_entry(&entry) {
                        Ok(())  => { commands.insert(name, entry); }
                        Err(e)  => eprintln!("{}", yellow(&format!("Warning: skipping invalid command '{name}': {e}"))),
                    }
                }
            }
            Err(e) => {
                eprintln!("{}", yellow(&format!("Config parse error at '{}': {e}", path.display())));
                eprintln!("{}", dim("Falling back to default configuration."));
            }
        }
    } else if let Err(e) = write_config_atomically(&path, default_config_toml()) {
        eprintln!("{}", yellow(&format!("Warning: could not write default config: {e}")));
    }

    let mut seen: HashMap<String, String> = HashMap::new();
    for (name, entry) in &commands {
        if let Some(prior) = seen.insert(entry.key.clone(), name.to_owned()) {
            eprintln!("{}", red(&format!(
                "Error: duplicate key '{}' in '{name}' (already used by '{prior}')", entry.key
            )));
            exit(1);
        }
    }

    Config { commands }
}

fn build_key_map(cfg: &Config) -> HashMap<String, CommandEntry> {
    cfg.commands.values().map(|e| (e.key.clone(), e.clone())).collect()
}

// ── Editor type alias ────────────────────────────────────────────────────────

type Rl = Editor<PackageCompleter, DefaultHistory>;

fn make_editor() -> Rl {
    let mut rl = Editor::new().expect("failed to create editor");
    rl.set_helper(Some(PackageCompleter::default()));
    rl
}

// ── Entry point ──────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(String::as_str) == Some("--version") {
        display_version();
        return;
    }

    let cfg    = load_config();
    let keymap = build_key_map(&cfg);

    if matches!(args.get(1).map(String::as_str), Some("-e") | Some("--exec")) {
        handle_exec(&args[2..], &cfg);
        return;
    }

    let mut rl = make_editor();
    println!("\n{} {}", bold(&cyan("Archie")), dim(&format!("v{VERSION} — type {} for help", bold("h"))));

    loop {
        match rl.readline(&format!("{} ", cyan("❯"))) {
            Ok(line) => {
                let key = line.trim();
                if key.is_empty() { continue; }
                rl.add_history_entry(key).ok();
                match keymap.get(key) {
                    Some(entry) => run_entry(entry, None, &mut rl, &keymap),
                    None        => println!("{}", dim("unknown command — type 'h' for help")),
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(e) => { eprintln!("Error: {e:?}"); break; }
        }
    }
    println!();
}

// ── Command dispatch ─────────────────────────────────────────────────────────

fn run_entry(
    entry:  &CommandEntry,
    arg:    Option<&str>,
    rl:     &mut Rl,
    keymap: &HashMap<String, CommandEntry>,
) {
    match entry.action.as_str() {
        "builtin:quit" => { println!(); exit(0); }
        "builtin:help" => { print_help(keymap); }
        _ => {
            let val: String = match (&entry.prompt, arg) {
                (None,    _       ) => String::new(),
                (Some(_), Some(a) ) => a.to_string(),
                (Some(p), None    ) => {
                    match rl.readline(&format!("{} ", cyan(&format!("{p} ❯")))) {
                        Ok(s) => {
                            let s = s.trim().to_string();
                            if s.is_empty() { return; }
                            rl.add_history_entry(&s).ok();
                            s
                        }
                        Err(_) => return,
                    }
                }
            };

            if entry.confirm.unwrap_or(false) && !ask_confirm(&val) { return; }

            let action = match &entry.prompt {
                Some(p) => entry.action.replace(&format!("{{{p}}}"), &val),
                None    => entry.action.clone(),
            };

            println!("{}", dim(&format!("→ {}...", entry.desc.as_deref().unwrap_or(&action))));
            dispatch(&action);
        }
    }
}

fn handle_exec(args: &[String], cfg: &Config) {
    let Some(name) = args.first() else {
        eprintln!("{}", dim("error: -e requires a command name"));
        exit(1);
    };
    let entry = cfg.commands.get(name.as_str()).unwrap_or_else(|| {
        eprintln!("{}", red(&format!("error: unknown command '{name}'")));
        exit(1);
    });
    let keymap = build_key_map(cfg);
    run_entry(entry, args.get(1).map(String::as_str), &mut make_editor(), &keymap);
}

fn ask_confirm(ctx: &str) -> bool {
    let label = if ctx.is_empty() {
        format!("{} ", bold(&red("confirm? [y/N]")))
    } else {
        format!("{} ", bold(&red(&format!("confirm '{ctx}'? [y/N]"))))
    };
    print!("{label}");
    let _ = io::stdout().flush();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
    matches!(buf.trim().to_lowercase().as_str(), "y" | "yes")
}

fn dispatch(action: &str) {
    if let Some(cmd) = action.strip_prefix("shell:") {
        let _ = Command::new("sh")
            .args(["-c", cmd])
            .stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit())
            .status();
    } else {
        let parts: Vec<&str> = action.split_whitespace().collect();
        if parts.is_empty() { return; }
        let _ = Command::new(if parts[0] == "paru" { PARU } else { parts[0] })
            .args(&parts[1..])
            .stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit())
            .status();
    }
}

// ── Help / version ───────────────────────────────────────────────────────────

fn print_help(keymap: &HashMap<String, CommandEntry>) {
    let mut entries: Vec<&CommandEntry> = keymap.values().collect();
    entries.sort_by_key(|e| &e.key);

    let max_desc = entries.iter()
        .map(|e| e.desc.as_deref().unwrap_or(&e.action).len())
        .max()
        .unwrap_or(0);
    let width = (9 + max_desc).clamp(34, 72);

    println!("\n  {}   {}", bold(&cyan(&format!("{:<4}", "key"))), dim("description"));
    println!("  {}", dim(&"─".repeat(width)));
    for e in entries {
        println!("  {}   {}", bold(&cyan(&format!("{:<4}", e.key))), dim(e.desc.as_deref().unwrap_or(&e.action)));
    }
    println!();
}

fn display_version() {
    let paru_ver = Command::new(PARU).arg("--version").output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            let mut parts = s.lines().next()?.split_whitespace().skip(1);
            Some(format!("paru {}", parts.next()?))
        })
        .unwrap_or_else(|| "paru unknown".into());

    println!("    __     ");
    println!(" .:--.'.   {} {}", bold(&cyan("Archie-ng")), bold(&format!("v{VERSION}")));
    println!("/ |   \\ |  {}", dim("Fast & easy package management for Arch Linux"));
    println!("`\" __ | |  {}", dim(&paru_ver));
    println!(" .'.''| |  ");
    println!("/ /   | |_ {}", dim("This program may be freely redistributed under the terms of the GNU General Public License."));
    println!("\\ \\._,\\ '/ {}", dim("Created & maintained by Gurov"));
    println!(" `--'  `\"  ");
}

// ── Tab completion ───────────────────────────────────────────────────────────

#[derive(Default)]
struct PackageCompleter {
    cache: OnceLock<Vec<String>>,
}

impl PackageCompleter {
    fn packages(&self) -> &[String] {
        self.cache.get_or_init(load_packages)
    }
}

impl Helper    for PackageCompleter {}
impl Hinter    for PackageCompleter { type Hint = String; }
impl Validator for PackageCompleter {}
impl Highlighter for PackageCompleter {}

impl Completer for PackageCompleter {
    type Candidate = String;

    fn complete(&self, line: &str, _pos: usize, _ctx: &Context<'_>)
        -> Result<(usize, Vec<String>), ReadlineError>
    {
        let pkgs = self.packages();
        let start = pkgs.partition_point(|p| p.as_str() < line);
        let matches = pkgs[start..].iter()
            .take_while(|p| p.starts_with(line))
            .cloned()
            .collect();
        Ok((0, matches))
    }
}

fn load_packages() -> Vec<String> {
    let mut packages = Vec::with_capacity(8_192);

    if let Ok(out) = Command::new("pacman").arg("-Slq").output() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            packages.extend(s.lines().filter(|l| !l.is_empty()).map(str::to_string));
        }
    }

    let aur_cache = cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("paru/packages.aur");

    if let Ok(content) = fs::read_to_string(aur_cache) {
        packages.extend(content.lines().filter(|l| !l.is_empty()).map(str::to_string));
    }

    packages.sort_unstable();
    packages.dedup();
    packages
}
