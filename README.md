# Archie

> A minimal, blazing-fast interactive wrapper for `paru` on Arch Linux.

## Features

- **Simple Interface** - Single-letter commands for common package management tasks
- **Zero Dependencies** - Built with Rust stdlib only, no external crates
- **Instant Startup** - Optimized release build with LTO and `panic = "abort"`
- **Clean Code** - Minimal, comment-free, production-ready Rust

## Installation

### Prerequisites

- Rust toolchain (`rustup`)
- `paru` package manager installed

### Build from Source

```bash
git clone https://github.com/Gur0v/archie-ng
cd archie
cargo build --release
sudo cp target/release/archie /usr/local/bin/
```

### Optional: Install via AUR

```bash
paru -S archie
```

## Usage

### Interactive Mode

```bash
$ archie
Welcome to Archie v3.0.0
Using paru package manager
Type 'h' for help

$ u    # Update system
$ i    # Install package
$ r    # Remove package
$ p    # Purge package + dependencies
$ s    # Search packages
$ c    # Clean cache
$ o    # Remove orphans
$ h    # Show help
$ q    # Quit
```

### Command Line

```bash
archie --version    # Show version
```

## Commands

| Key | Action   | Description                          |
|-----|----------|--------------------------------------|
| `u` | Update   | Run `paru -Syu`                      |
| `i` | Install  | Run `paru -S <package>`              |
| `r` | Remove   | Run `paru -R <package>`              |
| `p` | Purge    | Run `paru -Rns <package>`            |
| `s` | Search   | Run `paru -Ss <query>`               |
| `c` | Clean    | Run `paru -Sc`                       |
| `o` | Orphans  | Remove orphaned dependencies         |
| `h` | Help     | Display command reference            |
| `q` | Quit     | Exit interactive mode                |

## License

GPL-3.0 License — see [LICENSE](LICENSE) for details.

---

*Archie is not affiliated with Arch Linux, paru, or the Arch Wiki.*
