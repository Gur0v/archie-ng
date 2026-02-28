use std::fs;
use std::path::PathBuf;
use std::process::Command;
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper, Editor, history::DefaultHistory};

const VERSION: &str = "3.1.0";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() == 2 && args[1] == "--version" {
        display_version();
        return;
    }
    
    let packages = load_packages();
    let mut rl: Editor<PackageCompleter, DefaultHistory> = Editor::new().expect("Failed to create editor");
    rl.set_helper(Some(PackageCompleter { packages }));
    
    println!("\nWelcome to Archie v{}", VERSION);
    println!("Using paru package manager");
    println!("Type 'h' for help\n");
    
    loop {
        let input: String = match rl.readline("$ ") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => break,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        };
        
        rl.add_history_entry(&input).expect("Failed to add history");
        
        match input.trim() {
            "u" => exec(&["-Syu"]),
            "i" => prompt_install(&mut rl),
            "r" => prompt_remove(&mut rl, "-R"),
            "p" => prompt_remove(&mut rl, "-Rns"),
            "s" => prompt_search(&mut rl),
            "c" => exec(&["-Sc"]),
            "o" => exec_shell("paru -Rns $(pacman -Qtdq)"),
            "h" => show_help(),
            "q" => break,
            "" => continue,
            _ => println!("Unknown command. Type 'h' for help"),
        }
    }
    println!();
}

fn display_version() {
    println!("    __     ");
    println!(" .:--.'.   Archie-ng v{} - Fast & Easy package management for Arch Linux", VERSION);
    println!("/ |   \\ |  Written in Rust, powered by paru.");
    println!("`\" __ | |  {}", get_paru_version());
    println!(" .'.''| |  ");
    println!("/ /   | |_ This program may be freely redistributed under the terms of the GNU General Public License.");
    println!("\\ \\._,\\ '/ Created & maintained by Gurov");
    println!(" `--'  `\"  ");
}

fn get_paru_version() -> String {
    if let Ok(output) = Command::new("paru").arg("--version").output() {
        if let Ok(line) = String::from_utf8(output.stdout) {
            if let Some(version) = line.lines().next() {
                return version.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
            }
        }
    }
    String::from("unknown")
}

fn load_packages() -> Vec<String> {
    let mut packages = Vec::new();
    
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
    if let Ok(content) = fs::read_to_string(&aur_cache) {
        for line in content.lines() {
            if !line.is_empty() {
                packages.push(line.to_string());
            }
        }
    }
    
    packages.sort();
    packages.dedup();
    packages
}

fn exec(args: &[&str]) {
    let _ = Command::new("paru").args(args).status();
}

fn exec_shell(cmd: &str) {
    let _ = Command::new("sh").arg("-c").arg(cmd).status();
}

fn prompt_install(rl: &mut Editor<PackageCompleter, DefaultHistory>) {
    let pkg: String = match rl.readline("Package: ") {
        Ok(line) => line,
        Err(_) => return,
    };
    rl.add_history_entry(&pkg).expect("Failed to add history");
    let pkg = pkg.trim();
    if !pkg.is_empty() {
        exec(&["-S", pkg]);
    }
}

fn prompt_remove(rl: &mut Editor<PackageCompleter, DefaultHistory>, flag: &str) {
    let pkg: String = match rl.readline("Package: ") {
        Ok(line) => line,
        Err(_) => return,
    };
    rl.add_history_entry(&pkg).expect("Failed to add history");
    let pkg = pkg.trim();
    if !pkg.is_empty() {
        exec(&[flag, pkg]);
    }
}

fn prompt_search(rl: &mut Editor<PackageCompleter, DefaultHistory>) {
    let query: String = match rl.readline("Search: ") {
        Ok(line) => line,
        Err(_) => return,
    };
    rl.add_history_entry(&query).expect("Failed to add history");
    let query = query.trim();
    if !query.is_empty() {
        exec(&["-Ss", query]);
    }
}

fn show_help() {
    println!("\nCommands:");
    println!("  u - Update system      i - Install package");
    println!("  r - Remove package     p - Purge package");
    println!("  s - Search packages    c - Clean cache");
    println!("  o - Remove orphans     h - Show help");
    println!("  q - Quit\n");
}

struct PackageCompleter {
    packages: Vec<String>,
}

impl Helper for PackageCompleter {}

impl Completer for PackageCompleter {
    type Candidate = String;
    
    fn complete(&self, line: &str, _pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<String>), ReadlineError> {
        let matches: Vec<String> = self.packages
            .iter()
            .filter(|pkg| pkg.starts_with(line))
            .cloned()
            .collect();
        Ok((0, matches))
    }
}

impl Hinter for PackageCompleter {
    type Hint = String;
}

impl Validator for PackageCompleter {}

impl Highlighter for PackageCompleter {}
