# Archie

> Even faster & easier package management for Arch Linux.

A ground-up Rust rewrite of [TuxForge/archie](https://github.com/TuxForge/archie) — single-letter commands, tab completion backed by sorted binary-search indexes, and safe subprocess dispatch with no shell injection surface.

## Why rewrite?

The original C version worked — until it didn't.

| | C v1.3 | Rust v3.7+ |
|---|---|---|
| Shell injection via `system()` | Possible | Prevented |
| Memory leaks | Present | Gone |
| Buffer / stack overflows | Possible | Gone |
| Tab completion lag | ~3s | Instant |
| Build setup | make + readline | `cargo build` |

## Installation

**AUR**

```bash
paru -S archie
```

**From source** — requires `rustup` and `paru`

```bash
git clone https://github.com/Gur0v/archie-ng
cd archie-ng
cargo build --release
sudo install -Dm755 target/release/archie /usr/local/bin/archie
```

## Usage

### Interactive mode

```
$ archie

Archie v3.7.0 — type h for help
❯ h

  key    description
  ──────────────────────────────────
  c      clean package cache
  h      show this help
  i      install a package
  l      view recent pacman log
  o      remove orphaned packages
  p      remove package + deps
  q      exit archie
  r      remove a package
  s      search packages
  u      upgrade all packages
```

Type a key to run a command. At any prompt:

- **Tab** — complete package names
- **↑ / ↓** — history navigation
- **← / →** — cursor movement
- **Ctrl+A / Ctrl+E** — jump to start / end of line
- **Ctrl+C / Ctrl+D** — exit

### Exec mode

Run any command non-interactively, useful for scripts and shell aliases:

```bash
archie -e install firefox   # argument supplied directly
archie -e install           # prompts for input with completion
archie -e update
archie -e orphans
```

Exit codes: `0` on success, `1` on invalid command.

## Configuration

Archie reads `~/.config/archie/archie.toml` on startup, writing the default config if none exists. The file is written atomically via a `.tmp` rename to prevent corruption on interrupted writes.

```toml
edition = "2026-1"

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
```

The `edition` field signals config compatibility. A mismatch produces a warning at startup but does not prevent Archie from running. Duplicate keys across commands are detected at load time and cause an immediate exit with a descriptive error.

### Command fields

| Field | Required | Description |
|---|---|---|
| `key` | yes | Single-character trigger |
| `action` | yes | Command to execute |
| `desc` | no | Text shown in the help screen |
| `prompt` | no | Prompts for input and fills `{placeholder}` in `action` |
| `confirm` | no | Requires `y/N` confirmation before executing |

### Action prefixes

| Prefix | Behavior |
|---|---|
| *(none)* | Spawned directly via `execv` — no shell, no injection surface |
| `shell:` | Passed to `sh -c` — supports pipes, redirects, and subshells |
| `builtin:quit` | Exit Archie |
| `builtin:help` | Show the help screen |

### Adding custom commands

```toml
[commands]
mirror = { key = "m", action = "shell:rate-mirrors --save /etc/pacman.d/mirrorlist arch", desc = "update mirrorlist"    }
diff   = { key = "d", action = "shell:pacdiff",                                           desc = "review pacdiff files" }
```

## Package cache

Tab completion is backed by two sorted, newline-delimited plain-text databases in `~/.cache/archie/`. Prefix lookups use `partition_point` for O(log n) binary search — no fuzzy matching overhead, no startup latency.

| File | Populated by | Used for |
|---|---|---|
| `available.db` | `paru -Slq` | `install`, `search` prompts |
| `installed.db` | `paru -Qq` | `remove`, `purge` prompts |

Both files are created on first launch if absent. After any install, remove, or update action they are refreshed in a background thread behind an `Arc<RwLock<_>>`, keeping completion current for the next session without blocking the UI.

## License

[GPL-3.0](LICENSE) — not affiliated with Arch Linux or paru.
