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

const VERSION: &str = "3.3.0";

static PARU_PATH: OnceLock<PathBuf> = OnceLock::new();

fn paru_path() -> &'static Path {
    PARU_PATH.get_or_init(|| PathBuf::from("paru"))
}

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

    println!("\nWelcome to Archie v{VERSION}");
    println!("Type 'h' for help\n");

    loop {
        let input = match rl.readline("$ ") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(e) => { eprintln!("Error: {e:?}"); break; }
        };

        rl.add_history_entry(&input).expect("Failed to add history");

        match input.trim() {
            "u" => paru(&["-Syu"]),
            "i" => prompt(&mut rl, "Package: ", |p| paru(&["-S", p])),
            "r" => prompt(&mut rl, "Package: ", |p| paru(&["-R", p])),
            "p" => prompt(&mut rl, "Package: ", |p| paru(&["-Rns", p])),
            "s" => prompt(&mut rl, "Search: ",  |p| paru(&["-Ss", p])),
            "c" => paru(&["-Sc"]),
            "o" => shell("paru -Rns $(pacman -Qtdq)"),
            "h" => {
                println!("\nCommands:");
                println!("  u - Update system      i - Install package");
                println!("  r - Remove package     p - Purge package");
                println!("  s - Search packages    c - Clean cache");
                println!("  o - Remove orphans     h - Show help");
                println!("  q - Quit\n");
            }
            "q" => break,
            "" => continue,
            _ => println!("Unknown command. Type 'h' for help"),
        }
    }
    println!();
}

#[inline]
fn handle_exec(args: &[String]) {
    let Some(cmd) = args.first() else {
        eprintln!("Error: -e requires a command (u|i|r|p|c|o|s|h)");
        exit(1);
    };

    let extra = &args[1..];
    
    match cmd.as_str() {
        "u" => paru(&["-Syu"]),
        "i" => exec_with_prompt("Package: ", |p| paru(&["-S", p]), extra),
        "r" => exec_with_prompt("Package: ", |p| paru(&["-R", p]), extra),
        "p" => exec_with_prompt("Package: ", |p| paru(&["-Rns", p]), extra),
        "c" => paru(&["-Sc"]),
        "o" => shell("paru -Rns $(pacman -Qtdq)"),
        "s" => exec_with_prompt("Search: ", |q| paru(&["-Ss", q]), extra),
        "h" => println!("Commands: u - Update, i - Install, r - Remove, p - Purge, s - Search, c - Clean cache, o - Remove orphans, h - Help, q - Quit"),
        _ => {
            eprintln!("Invalid command for -e: {cmd}");
            eprintln!("Valid commands: u|i|r|p|c|o|s|h");
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
    println!(" .:--.'.   Archie-ng v{VERSION} - Fast & Easy package management for Arch Linux");
    println!("/ |   \\ |  Written in Rust, powered by paru.");
    println!("`\" __ | |  {paru_ver}");
    println!(" .'.''| |  ");
    println!("/ /   | |_ This program may be freely redistributed under the terms of the GNU General Public License.");
    println!("\\ \\._,\\ '/ Created & maintained by Gurov");
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
