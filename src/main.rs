use std::io::{self, Write};
use std::process::Command;

const VERSION: &str = "3.0.0";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() == 2 && args[1] == "--version" {
        println!("Archie v{}", VERSION);
        return;
    }
    
    println!("\nWelcome to Archie v{}", VERSION);
    println!("Using paru package manager");
    println!("Type 'h' for help\n");
    
    loop {
        print!("$ ");
        let _ = io::stdout().flush();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        
        match input.trim() {
            "u" => exec(&["-Syu"]),
            "i" => prompt_install(),
            "r" => prompt_remove("-R"),
            "p" => prompt_remove("-Rns"),
            "s" => prompt_search(),
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

fn exec(args: &[&str]) {
    let _ = Command::new("paru").args(args).status();
}

fn exec_shell(cmd: &str) {
    let _ = Command::new("sh").arg("-c").arg(cmd).status();
}

fn prompt_install() {
    print!("Package: ");
    let _ = io::stdout().flush();
    let mut pkg = String::new();
    if io::stdin().read_line(&mut pkg).is_ok() {
        let pkg = pkg.trim();
        if !pkg.is_empty() {
            exec(&["-S", pkg]);
        }
    }
}

fn prompt_remove(flag: &str) {
    print!("Package: ");
    let _ = io::stdout().flush();
    let mut pkg = String::new();
    if io::stdin().read_line(&mut pkg).is_ok() {
        let pkg = pkg.trim();
        if !pkg.is_empty() {
            exec(&[flag, pkg]);
        }
    }
}

fn prompt_search() {
    print!("Search: ");
    let _ = io::stdout().flush();
    let mut query = String::new();
    if io::stdin().read_line(&mut query).is_ok() {
        let query = query.trim();
        if !query.is_empty() {
            exec(&["-Ss", query]);
        }
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
