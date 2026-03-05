use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, exit};
use std::sync::{Arc, RwLock};
use std::thread;

use dirs::{cache_dir, config_dir};
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};
use serde::Deserialize;

const VERSION:        &str = "3.7.0";
const CONFIG_EDITION: &str = "2026-1";
const PARU:           &str = "paru";

const DEFAULT_CONFIG: &str = r#"edition = "2026-1"

[commands]
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
"#;

macro_rules! ansi {
    ($code:literal, $name:ident) => {
        fn $name(s: &str) -> String {
            format!(concat!("\x1b[", $code, "m{}\x1b[0m"), s)
        }
    };
}

ansi!("1",  bold);
ansi!("2",  dim);
ansi!("31", red);
ansi!("33", yellow);
ansi!("36", cyan);

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
    edition:  Option<String>,
    commands: HashMap<String, CommandEntry>,
}

impl Config {
    fn load() -> Self {
        let path = Self::path();
        let mut commands = toml::from_str::<Config>(DEFAULT_CONFIG)
            .expect("built-in config must be valid")
            .commands;

        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<Config>(&content) {
                Ok(user) => {
                    if user.edition.as_deref() != Some(CONFIG_EDITION) {
                        eprintln!("{}", yellow(&format!(
                            "Warning: config edition '{}' differs from current '{}' — some options may be ignored.",
                            user.edition.as_deref().unwrap_or("unset"),
                            CONFIG_EDITION,
                        )));
                    }
                    for (name, entry) in user.commands {
                        match Self::validate(&entry) {
                            Ok(())  => { commands.insert(name, entry); }
                            Err(e)  => eprintln!("{}", yellow(&format!("Warning: skipping invalid command '{name}': {e}"))),
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", yellow(&format!("Config parse error at '{}': {e}", path.display())));
                    eprintln!("{}", dim("Falling back to default configuration."));
                }
            },
            Err(_) => {
                if let Err(e) = Self::write_default(&path) {
                    eprintln!("{}", yellow(&format!("Warning: could not write default config: {e}")));
                }
            }
        }

        let mut seen: HashMap<String, String> = HashMap::new();
        for (name, entry) in &commands {
            if let Some(prior) = seen.insert(entry.key.clone(), name.clone()) {
                eprintln!("{}", red(&format!(
                    "Error: duplicate key '{}' in '{name}' (already used by '{prior}')", entry.key
                )));
                exit(1);
            }
        }

        Self { edition: Some(CONFIG_EDITION.into()), commands }
    }

    fn path() -> PathBuf {
        config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("archie")
            .join("archie.toml")
    }

    fn validate(entry: &CommandEntry) -> Result<(), String> {
        if entry.key.is_empty()    { return Err("key is empty".into()); }
        if entry.action.is_empty() { return Err("action is empty".into()); }
        if let Some(p) = &entry.prompt {
            if !entry.action.contains(&format!("{{{p}}}")) {
                return Err(format!("action missing placeholder {{{p}}}"));
            }
        }
        Ok(())
    }

    fn write_default(path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, DEFAULT_CONFIG)?;
        fs::rename(&tmp, path)
    }

    fn key_map(&self) -> HashMap<String, CommandEntry> {
        self.commands.values().map(|e| (e.key.clone(), e.clone())).collect()
    }
}

struct PackageDb {
    available: Arc<RwLock<Vec<String>>>,
    installed: Arc<RwLock<Vec<String>>>,
}

impl PackageDb {
    fn load() -> Self {
        Self {
            available: Arc::new(RwLock::new(Self::load_or_fetch(
                Self::available_path(),
                || Self::fetch_packages(PARU, &["-Slq"]),
            ))),
            installed: Arc::new(RwLock::new(Self::load_or_fetch(
                Self::installed_path(),
                || Self::fetch_packages(PARU, &["-Qq"]),
            ))),
        }
    }

    fn refresh_in_background(&self) {
        let av = Arc::clone(&self.available);
        let iv = Arc::clone(&self.installed);
        thread::spawn(move || {
            let fresh_av = Self::fetch_and_cache(Self::available_path(), PARU, &["-Slq"]);
            let fresh_iv = Self::fetch_and_cache(Self::installed_path(), PARU, &["-Qq"]);
            if let Ok(mut w) = av.write() { *w = fresh_av; }
            if let Ok(mut w) = iv.write() { *w = fresh_iv; }
        });
    }

    fn cache_dir() -> PathBuf {
        cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("archie")
    }

    fn available_path() -> PathBuf { Self::cache_dir().join("available.db") }
    fn installed_path() -> PathBuf { Self::cache_dir().join("installed.db") }

    fn load_or_fetch(path: PathBuf, fetch: impl FnOnce() -> Vec<String>) -> Vec<String> {
        if path.exists() {
            Self::read_file(&path)
        } else {
            fetch()
        }
    }

    fn read_file(path: &Path) -> Vec<String> {
        fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect()
    }

    fn fetch_packages(bin: &str, args: &[&str]) -> Vec<String> {
        let mut pkgs: Vec<String> = Command::new(bin)
            .args(args)
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.lines().filter(|l| !l.is_empty()).map(str::to_string).collect())
            .unwrap_or_default();
        pkgs.sort_unstable();
        pkgs.dedup();
        pkgs
    }

    fn fetch_and_cache(path: PathBuf, bin: &str, args: &[&str]) -> Vec<String> {
        let pkgs = Self::fetch_packages(bin, args);
        let _ = fs::create_dir_all(path.parent().unwrap());
        let _ = fs::write(&path, pkgs.join("\n"));
        pkgs
    }
}

#[derive(Clone, Copy, PartialEq)]
enum CompleteMode {
    Available,
    Installed,
    None,
}

struct PackageCompleter {
    available: Arc<RwLock<Vec<String>>>,
    installed: Arc<RwLock<Vec<String>>>,
    mode:      CompleteMode,
}

impl PackageCompleter {
    fn db(&self) -> std::sync::RwLockReadGuard<'_, Vec<String>> {
        match self.mode {
            CompleteMode::Installed => self.installed.read().unwrap(),
            _                       => self.available.read().unwrap(),
        }
    }
}

impl Helper     for PackageCompleter {}
impl Validator  for PackageCompleter {}
impl Highlighter for PackageCompleter {}

impl Hinter for PackageCompleter {
    type Hint = String;

    fn hint(&self, line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        if line.is_empty() || self.mode == CompleteMode::None { return None; }
        let db  = self.db();
        let idx = db.partition_point(|p| p.as_str() < line);
        db[idx..].iter()
            .find(|p| p.starts_with(line))
            .map(|p| dim(&p[line.len()..]))
    }
}

impl Completer for PackageCompleter {
    type Candidate = String;

    fn complete(&self, line: &str, _pos: usize, _ctx: &Context<'_>)
        -> Result<(usize, Vec<String>), ReadlineError>
    {
        if self.mode == CompleteMode::None { return Ok((0, vec![])); }
        let db    = self.db();
        let start = db.partition_point(|p| p.as_str() < line);
        let hits  = db[start..].iter().take_while(|p| p.starts_with(line)).cloned().collect();
        Ok((0, hits))
    }
}

type Rl = Editor<PackageCompleter, DefaultHistory>;

fn make_editor(db: &PackageDb) -> Rl {
    let mut rl = Editor::new().expect("failed to create line editor");
    rl.set_helper(Some(PackageCompleter {
        available: Arc::clone(&db.available),
        installed: Arc::clone(&db.installed),
        mode:      CompleteMode::None,
    }));
    rl
}

fn run_entry(
    entry:  &CommandEntry,
    arg:    Option<&str>,
    rl:     &mut Rl,
    keymap: &HashMap<String, CommandEntry>,
    db:     &PackageDb,
) {
    match entry.action.as_str() {
        "builtin:quit" => { println!(); exit(0); }
        "builtin:help" => { print_help(keymap); }
        _ => {
            if let Some(h) = rl.helper_mut() {
                h.mode = match entry.prompt.as_deref() {
                    Some("pkg") if entry.action.contains("-R") => CompleteMode::Installed,
                    Some("pkg")                                => CompleteMode::Available,
                    _                                          => CompleteMode::None,
                };
            }

            let val: String = match (&entry.prompt, arg) {
                (None,    _       ) => String::new(),
                (Some(_), Some(a) ) => a.to_string(),
                (Some(p), None    ) => match rl.readline(&format!("{} ", cyan(&format!("{p} ❯")))) {
                    Ok(s) => {
                        let s = s.trim().to_string();
                        if s.is_empty() { return; }
                        rl.add_history_entry(&s).ok();
                        s
                    }
                    Err(_) => return,
                },
            };

            if entry.confirm.unwrap_or(false) && !confirm(&val) { return; }

            let action = match &entry.prompt {
                Some(p) => entry.action.replace(&format!("{{{p}}}"), &val),
                None    => entry.action.clone(),
            };

            println!("{}", dim(&format!("→ {}...", entry.desc.as_deref().unwrap_or(&action))));
            dispatch(&action);

            if needs_refresh(&action) {
                db.refresh_in_background();
            }
        }
    }
}

fn needs_refresh(action: &str) -> bool {
    matches!(action, "paru -Syu" | "paru -Sc")
        || action.starts_with("paru -S ")
        || action.starts_with("paru -R")
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

fn confirm(ctx: &str) -> bool {
    let prompt = if ctx.is_empty() {
        format!("{} ", bold(&red("confirm? [y/N]")))
    } else {
        format!("{} ", bold(&red(&format!("confirm '{ctx}'? [y/N]"))))
    };
    print!("{prompt}");
    let _ = io::stdout().flush();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
    matches!(buf.trim().to_lowercase().as_str(), "y" | "yes")
}

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
    for e in &entries {
        println!("  {}   {}", bold(&cyan(&format!("{:<4}", e.key))), dim(e.desc.as_deref().unwrap_or(&e.action)));
    }
    println!();
}

fn print_version() {
    let paru_ver = Command::new(PARU)
        .arg("--version")
        .output()
        .ok()
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

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(String::as_str) == Some("--version") {
        print_version();
        return;
    }

    let cfg    = Config::load();
    let keymap = cfg.key_map();
    let db     = PackageDb::load();

    if matches!(args.get(1).map(String::as_str), Some("-e") | Some("--exec")) {
        let Some(name) = args.get(2) else {
            eprintln!("{}", dim("error: -e requires a command name"));
            exit(1);
        };
        let entry = cfg.commands.get(name.as_str()).unwrap_or_else(|| {
            eprintln!("{}", red(&format!("error: unknown command '{name}'")));
            exit(1);
        });
        run_entry(entry, args.get(3).map(String::as_str), &mut make_editor(&db), &keymap, &db);
        return;
    }

    let mut rl = make_editor(&db);
    println!("\n{} {}", bold(&cyan("Archie")), dim(&format!("v{VERSION} — type {} for help", bold("h"))));

    loop {
        match rl.readline(&format!("{} ", cyan("❯"))) {
            Ok(line) => {
                let key = line.trim();
                if key.is_empty() { continue; }
                rl.add_history_entry(key).ok();
                match keymap.get(key) {
                    Some(entry) => run_entry(entry, None, &mut rl, &keymap, &db),
                    None        => println!("{}", dim("unknown command — type 'h' for help")),
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(e) => { eprintln!("Error: {e:?}"); break; }
        }
    }
    println!();
}
