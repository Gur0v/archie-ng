# Archie

> Even faster & easier package management for Arch Linux.

A minimal interactive wrapper for `paru`, rewritten in Rust.

## Features

- Single-letter commands for common operations
- No external crates — Rust stdlib only
- No shell injection — explicit `Command::new()` args throughout
- ~4x faster startup than the previous C version (0.8ms vs 3.4ms)
- 420KB static binary

## Installation

### AUR

```bash
paru -S archie
```

### Build from source

```bash
git clone https://github.com/Gur0v/archie-ng
cd archie-ng
cargo build --release
sudo cp target/release/archie /usr/local/bin/
```

**Prerequisites:** Rust toolchain (`rustup`), `paru`

## Usage

```
$ archie

Welcome to Archie v3.0.0
Using paru package manager
Type 'h' for help

$ u    # Update system         → paru -Syu
$ i    # Install package       → paru -S <pkg>
$ r    # Remove package        → paru -R <pkg>
$ p    # Purge + deps          → paru -Rns <pkg>
$ s    # Search                → paru -Ss <query>
$ c    # Clean cache           → paru -Sc
$ o    # Remove orphans        → paru -Rns $(pacman -Qtdq)
$ h    # Help
$ q    # Quit
```

```bash
archie --version
```

## Why rewrite in Rust?

The C version had a few fundamental issues that warranted a full rewrite rather than patches.

**Shell injection** — package names were interpolated directly into `system()` calls with no sanitization.

**Memory leaks** — `strdup()` was called repeatedly with no corresponding `free()`.

**Stack overflow on empty input** — `get_input()` called itself recursively on empty input. Enough Enter presses would segfault the process.

**Slow tab completion** — every Tab press spawned a fresh `pacman -Ssq` process, taking 2–5 seconds each time.

| | C v1.3 | Rust v3.0 |
|---|---|---|
| Shell injection | possible | prevented |
| Memory leaks | present | eliminated |
| Stack overflow on empty input | yes | no |
| Startup time | ~3.4ms | ~0.8ms |
| Binary size | ~50KB | ~420KB |
| Build | `make` + readline dep | `cargo build` |

## License

GPL-3.0 — see [LICENSE](LICENSE) for details.

---

*Not affiliated with Arch Linux, paru, or the Arch Wiki.*
