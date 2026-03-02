use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, exit};
use std::sync::OnceLock;
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Editor, history::DefaultHistory};

const VERSION: &str = "3.4.0";

static PARU_PATH: OnceLock<PathBuf> = OnceLock::new();

fn paru_path() -> &'static Path {
    PARU_PATH.get_or_init(|| PathBuf::from("paru"))
}

fn cyan(s: &str) -> String { format!("\x1b[36m{s}\x1b[0m") }
fn bold(s: &str) -> String { format!("\x1b[1m{s}\x1b[0m") }
fn dim(s: &str)  -> String { format!("\x1b[2m{s}\x1b[0m") }

fn main() {
    let _ = PARU_PATH.set(PathBuf::from("paru"));

    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(String::as_str) == Some("--version") {
        display_version();
        return;
    }

    if args.get(1).map(String::as_str) == Some("-e") || args.get(1).map(String::as_str) == Some("--exec") {
        handle_exec(&args[2..]);
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

        rl.add_history_entry(&input).expect("Failed to add history");

        match input.trim() {
            "u" => { println!("{}", dim("→ updating system...")); paru(&["-Syu"]); }
            "i" => prompt(&mut rl, &format!("{} ", cyan("pkg ❯")), |p| { println!("{}", dim(&format!("→ installing {p}..."))); paru(&["-S", p]); }),
            "r" => prompt(&mut rl, &format!("{} ", cyan("pkg ❯")), |p| { println!("{}", dim(&format!("→ removing {p}..."))); paru(&["-R", p]); }),
            "p" => prompt(&mut rl, &format!("{} ", cyan("pkg ❯")), |p| { println!("{}", dim(&format!("→ purging {p}..."))); paru(&["-Rns", p]); }),
            "s" => prompt(&mut rl, &format!("{} ", cyan("search ❯")), |p| { println!("{}", dim(&format!("→ searching {p}..."))); paru(&["-Ss", p]); }),
            "c" => { println!("{}", dim("→ cleaning cache...")); paru(&["-Sc"]); }
            "o" => { println!("{}", dim("→ removing orphans...")); shell("paru -Rns $(pacman -Qtdq)"); }
            "h" => print_help(),
            "q" => break,
            ""  => continue,
            _   => println!("{}", dim("unknown command — type 'h' for help")),
        }
    }
    println!();
}

fn help_row(key: &str, cmd: &str, desc: &str) {
    println!("  {}   {}   {}",
        bold(&cyan(&format!("{key:<3}"))),
        bold(&format!("{cmd:<7}")),
        dim(desc),
    );
}

fn print_help() {
    println!();
    println!("  {}   {}   {}", bold(&cyan(&format!("{:<3}", "key"))), bold(&format!("{:<7}", "command")), dim("description"));
    println!("  {}", dim("─────────────────────────────────────────"));
    help_row("u", "update",  "upgrade all packages");
    help_row("i", "install", "install a package");
    help_row("r", "remove",  "remove a package");
    help_row("p", "purge",   "remove package with deps");
    help_row("s", "search",  "search packages");
    help_row("c", "clean",   "clean package cache");
    help_row("o", "orphans", "remove orphaned packages");
    help_row("q", "quit",    "exit archie");
    println!();
}

#[inline]
fn handle_exec(args: &[String]) {
    let Some(cmd) = args.first() else {
        eprintln!("{}", dim("error: -e requires a command (u|i|r|p|c|o|s|h)"));
        exit(1);
    };

    let extra = &args[1..];

    match cmd.as_str() {
        "u" => { println!("{}", dim("→ updating system...")); paru(&["-Syu"]); }
        "i" => exec_with_prompt(&format!("{} ", cyan("pkg ❯")), |p| { println!("{}", dim(&format!("→ installing {p}..."))); paru(&["-S", p]); }, extra),
        "r" => exec_with_prompt(&format!("{} ", cyan("pkg ❯")), |p| { println!("{}", dim(&format!("→ removing {p}..."))); paru(&["-R", p]); }, extra),
        "p" => exec_with_prompt(&format!("{} ", cyan("pkg ❯")), |p| { println!("{}", dim(&format!("→ purging {p}..."))); paru(&["-Rns", p]); }, extra),
        "c" => { println!("{}", dim("→ cleaning cache...")); paru(&["-Sc"]); }
        "o" => { println!("{}", dim("→ removing orphans...")); shell("paru -Rns $(pacman -Qtdq)"); }
        "s" => exec_with_prompt(&format!("{} ", cyan("search ❯")), |q| { println!("{}", dim(&format!("→ searching {q}..."))); paru(&["-Ss", q]); }, extra),
        "h" => print_help(),
        _ => {
            eprintln!("{}", dim(&format!("error: invalid command '{cmd}' — valid: u|i|r|p|c|o|s|h")));
            exit(1);
        }
    }
}

#[inline]
fn exec_with_prompt<F>(label: &str, action: F, extra: &[String])
where
    F: Fn(&str),
{
    if let Some(arg) = extra.first() {
        action(arg);
    } else {
        let mut rl: Editor<PackageCompleter, DefaultHistory> = Editor::new().expect("Failed to create editor");
        rl.set_helper(Some(PackageCompleter { packages: load_packages() }));
        if let Ok(input) = rl.readline(label) {
            let val = input.trim();
            if !val.is_empty() {
                action(val);
            }
        }
    }
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

    let aur_cache = PathBuf::from(env!("HOME")).join(".cache/paru/packages.aur");
    if let Ok(file) = File::open(aur_cache) {
        let reader = BufReader::new(file);
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

#[inline]
fn paru(args: &[&str]) {
    let _ = Command::new(paru_path())
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
}

#[inline]
fn shell(cmd: &str) {
    let _ = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
}

fn prompt(rl: &mut Editor<PackageCompleter, DefaultHistory>, label: &str, action: impl Fn(&str)) {
    if let Ok(input) = rl.readline(label) {
        rl.add_history_entry(&input).expect("Failed to add history");
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            action(trimmed);
        }
    }
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
