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
use rustyline::{Context, Helper, Editor, history::DefaultHistory};
use serde::Deserialize;
use dirs::{config_dir, cache_dir};

const VERSION: &str = "3.5.0-rc2";

static PARU_PATH: OnceLock<PathBuf> = OnceLock::new();

fn paru_path() -> &'static std::path::Path {
    PARU_PATH.get_or_init(|| PathBuf::from("paru"))
}

fn cyan(s: &str) -> String { format!("\x1b[36m{s}\x1b[0m") }
fn bold(s: &str) -> String { format!("\x1b[1m{s}\x1b[0m") }
fn dim(s: &str)  -> String { format!("\x1b[2m{s}\x1b[0m") }
fn red(s: &str)  -> String { format!("\x1b[31m{s}\x1b[0m") }
fn yellow(s: &str) -> String { format!("\x1b[33m{s}\x1b[0m") }

#[derive(Deserialize, Clone)]
struct CommandEntry {
    key: String,
    action: String,
    prompt: Option<String>,
    desc: Option<String>,
    confirm: Option<bool>,
}

#[derive(Deserialize)]
struct Config {
    commands: HashMap<String, CommandEntry>,
}

fn default_config_toml() -> &'static str {
    r#"[commands]
update  = { key = "u", action = "paru -Syu",                          desc = "upgrade all packages" }
install = { key = "i", action = "paru -S {pkg}",                      desc = "install a package",        prompt = "pkg" }
remove  = { key = "r", action = "paru -R {pkg}",                      desc = "remove a package",         prompt = "pkg" }
purge   = { key = "p", action = "paru -Rns {pkg}",                    desc = "remove package + deps",    prompt = "pkg",    confirm = true }
search  = { key = "s", action = "paru -Ss {query}",                   desc = "search packages",          prompt = "query" }
clean   = { key = "c", action = "paru -Sc",                           desc = "clean package cache" }
orphans = { key = "o", action = "shell:paru -Rns $(pacman -Qtdq)",    desc = "remove orphaned packages", confirm = true }
log     = { key = "l", action = "shell:tail -n 50 /var/log/pacman.log | less", desc = "view recent pacman log" }
quit    = { key = "q", action = "builtin:quit",                       desc = "exit archie" }
help    = { key = "h", action = "builtin:help",                       desc = "show this help" }
"#
}

fn config_path() -> PathBuf {
    let mut p = config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    p.push("archie");
    p.push("archie.toml");
    p
}

fn validate_entry(_key: &str, entry: &CommandEntry) -> Result<(), String> {
    if entry.key.is_empty() {
        return Err("key is empty".into());
    }
    if entry.action.is_empty() {
        return Err("action is empty".into());
    }
    if let Some(prompt) = &entry.prompt {
        if !entry.action.contains(&format!("{{{prompt}}}")) {
            return Err(format!("action missing placeholder {{{prompt}}}"));
        }
    }
    Ok(())
}

fn write_config_atomically(path: &Path, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp_path = path.with_extension("toml.tmp");
    fs::write(&temp_path, content)?;
    fs::rename(&temp_path, path)?;
    Ok(())
}

fn load_config() -> Config {
    let path = config_path();
    let mut final_commands = toml::from_str::<Config>(default_config_toml())
        .expect("default config is valid")
        .commands;

    let mut user_key_map: HashMap<String, String> = HashMap::new();

    if let Ok(content) = fs::read_to_string(&path) {
        match toml::from_str::<Config>(&content) {
            Ok(user_cfg) => {
                for (name, entry) in user_cfg.commands {
                    match validate_entry(&name, &entry) {
                        Ok(_) => {
                            if let Some(existing_name) = user_key_map.get(&entry.key) {
                                eprintln!("{}", red(&format!("Error: Duplicate command key '{}' in '{}' (already defined in '{}')", entry.key, name, existing_name)));
                                exit(1);
                            }
                            user_key_map.insert(entry.key.clone(), name.clone());
                            final_commands.insert(name, entry);
                        }
                        Err(e) => {
                            eprintln!("{}", yellow(&format!("Warning: Skipping invalid command '{name}': {e}")));
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{}", yellow(&format!("Config parse error at '{}': {e}", path.display())));
                eprintln!("{}", dim("Falling back to default configuration."));
            }
        }
    } else {
        if let Err(e) = write_config_atomically(&path, default_config_toml()) {
            eprintln!("{}", yellow(&format!("Warning: Could not write default config: {e}")));
        }
    }

    let mut final_key_map: HashMap<String, String> = HashMap::new();
    for (name, entry) in &final_commands {
        if let Some(existing_name) = final_key_map.get(&entry.key) {
            if existing_name != name {
                eprintln!("{}", red(&format!("Error: Duplicate command key '{}' in '{}' (already defined in '{}')", entry.key, name, existing_name)));
                exit(1);
            }
        }
        final_key_map.insert(entry.key.clone(), name.clone());
    }

    Config { commands: final_commands }
}

fn build_key_map(cfg: &Config) -> HashMap<String, CommandEntry> {
    cfg.commands.values().map(|e| (e.key.clone(), e.clone())).collect()
}

fn main() {
    let _ = PARU_PATH.set(PathBuf::from("paru"));
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(String::as_str) == Some("--version") {
        display_version();
        return;
    }

    let cfg = load_config();
    let keymap = build_key_map(&cfg);

    if args.get(1).map(String::as_str) == Some("-e") || args.get(1).map(String::as_str) == Some("--exec") {
        handle_exec(&args[2..], &cfg);
        return;
    }

    let mut rl: Editor<PackageCompleter, DefaultHistory> = Editor::new().expect("Failed to create editor");
    rl.set_helper(Some(PackageCompleter { packages: load_packages() }));

    println!("\n{} {}", bold(&cyan("Archie")), dim(&format!("v{VERSION} — type {} for help", bold("h"))));

    loop {
        let input = match rl.readline(&format!("{} ", cyan("❯"))) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(e) => { eprintln!("Error: {e:?}"); break; }
        };

        let key = input.trim();
        if key.is_empty() { continue; }

        rl.add_history_entry(key).expect("Failed to add history");

        if let Some(entry) = keymap.get(key) {
            run_entry(entry, None, &mut rl, &keymap);
        } else {
            println!("{}", dim("unknown command — type 'h' for help"));
        }
    }
    println!();
}

fn run_entry(entry: &CommandEntry, arg: Option<&str>, rl: &mut Editor<PackageCompleter, DefaultHistory>, keymap: &HashMap<String, CommandEntry>) {
    match entry.action.as_str() {
        "builtin:quit" => {
            println!();
            exit(0);
        }
        "builtin:help" => {
            print_help_from_entry_map(keymap);
        }
        _ => {
            let val = if let Some(placeholder) = &entry.prompt {
                if let Some(a) = arg {
                    a.to_string()
                } else {
                    let label = format!("{} ", cyan(&format!("{placeholder} ❯")));
                    match rl.readline(&label) {
                        Ok(input) => {
                            let trimmed = input.trim().to_string();
                            if trimmed.is_empty() { return; }
                            rl.add_history_entry(&trimmed).ok();
                            trimmed
                        }
                        Err(_) => return,
                    }
                }
            } else {
                String::new()
            };

            if entry.confirm.unwrap_or(false) {
                if !ask_confirm(&val) { return; }
            }

            let action = if entry.prompt.is_some() {
                let placeholder = entry.prompt.as_deref().unwrap_or("");
                entry.action.replace(&format!("{{{placeholder}}}"), &val)
            } else {
                entry.action.clone()
            };

            let desc = entry.desc.as_deref().unwrap_or(&action);
            println!("{}", dim(&format!("→ {desc}...")));

            dispatch(&action);
        }
    }
}

fn ask_confirm(context: &str) -> bool {
    let label = if context.is_empty() {
        format!("{} ", bold(&red("confirm? [y/N]")))
    } else {
        format!("{} ", bold(&red(&format!("confirm '{context}'? [y/N]"))))
    };
    print!("{label}");
    let _ = io::stdout().flush();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
    matches!(buf.trim().to_lowercase().as_str(), "y" | "yes")
}

fn dispatch(action: &str) {
    if let Some(cmd) = action.strip_prefix("shell:") {
        shell(cmd);
    } else {
        let parts: Vec<&str> = action.split_whitespace().collect();
        if parts.is_empty() { return; }
        if parts[0] == "paru" {
            paru(&parts[1..]);
        } else {
            let _ = Command::new(parts[0])
                .args(&parts[1..])
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status();
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

    let arg = args.get(1).map(String::as_str);

    let mut rl: Editor<PackageCompleter, DefaultHistory> = Editor::new().expect("Failed to create editor");
    rl.set_helper(Some(PackageCompleter { packages: load_packages() }));

    let keymap = build_key_map(cfg);
    run_entry(entry, arg, &mut rl, &keymap);
}

fn print_help_from_entry_map(keymap: &HashMap<String, CommandEntry>) {
    let mut entries: Vec<&CommandEntry> = keymap.values().collect();
    entries.sort_by(|a, b| a.key.cmp(&b.key));

    println!();
    println!("  {}   {}", bold(&cyan(&format!("{:<4}", "key"))), dim("description"));

    let max_len = entries.iter()
        .map(|e| e.desc.as_deref().unwrap_or(&e.action).len())
        .max()
        .unwrap_or(0);
    
    let width = (9 + max_len).clamp(34, 64);
    println!("  {}", dim(&"─".repeat(width)));

    for e in entries {
        let desc = e.desc.as_deref().unwrap_or(&e.action);
        println!("  {}   {}", bold(&cyan(&format!("{:<4}", e.key))), dim(desc));
    }
    println!();
}

fn display_version() {
    let paru_ver = Command::new("paru")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(|l| {
            let mut parts = l.split_whitespace().skip(1);
            parts.next().map(|v| format!("paru {}", v)).unwrap_or_else(|| "paru unknown".into())
        }))
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

fn load_packages() -> Vec<String> {
    let mut packages = Vec::with_capacity(120_000);

    if let Ok(output) = Command::new("pacman").arg("-Slq").output() {
        if let Ok(content) = String::from_utf8(output.stdout) {
            for line in content.lines() {
                if !line.is_empty() {
                    packages.push(line.to_string());
                }
            }
        }
    }

    let aur_cache = cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("paru")
        .join("packages.aur");
    if let Ok(file) = std::fs::File::open(aur_cache) {
        use std::io::BufRead;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines().flatten() {
            if !line.is_empty() {
                packages.push(line);
            }
        }
    }

    packages.sort();
    packages.dedup();
    packages
}

fn paru(args: &[&str]) {
    let _ = Command::new(paru_path())
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
}

fn shell(cmd: &str) {
    let _ = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
}

struct PackageCompleter {
    packages: Vec<String>,
}

impl Helper for PackageCompleter {}
impl Hinter for PackageCompleter { type Hint = String; }
impl Validator for PackageCompleter {}
impl Highlighter for PackageCompleter {}

impl Completer for PackageCompleter {
    type Candidate = String;

    fn complete(&self, line: &str, _pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<String>), ReadlineError> {
        let start = self.packages.binary_search_by(|p| p.as_str().cmp(line)).unwrap_or_else(|i| i);
        let mut matches = Vec::with_capacity(64);
        for pkg in &self.packages[start..] {
            if pkg.starts_with(line) {
                matches.push(pkg.clone());
            } else {
                break;
            }
        }
        Ok((0, matches))
    }
}
