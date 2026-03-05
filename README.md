# Archie
> Even faster & easier package management for Arch Linux.

A ground-up Rust rewrite of [TuxForge/archie](https://github.com/TuxForge/archie). Single-letter commands, instant tab completion, no runtime surprises.

## Why rewrite?
The original C version worked — until it didn't.

| Issue | C v1.3 | Rust v3.6+ |
|---|---|---|
| Shell injection via `system()` | Possible | Prevented |
| Memory leaks | Present | Gone |
| Buffer/stack overflows | Possible | Gone |
| Tab completion lag | ~3s | Instant |
| Build setup | make + readline | cargo build |

## Installation
**AUR**
```bash
paru -S archie
```

**From source**
```bash
git clone https://github.com/Gur0v/archie-ng
cd archie-ng
cargo build --release
sudo install -Dm755 target/release/archie /usr/local/bin/archie
```
Requires `rustup` and `paru`.

## Usage
### Interactive
```
bash-5.3$ archie
Archie v3.6.0 — type h for help
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

Type a key. Tab completion works on package names automatically. The prompt supports history navigation (`↑`/`↓`), cursor movement (`←`/`→`), `Ctrl+C`/`Ctrl+D` to exit, and `Ctrl+A`/`Ctrl+E` to jump to the start/end of the line.

### Exec mode
For scripts and aliases:
```bash
archie -e install firefox   # non-interactive, arg supplied
archie -e install           # prompts with completion
archie -e update
archie -e orphans
```
Exit codes: `0` success, `1` invalid command.

## Configuration
Archie reads `~/.config/archie/archie.toml` on startup, creating it with defaults if absent.

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

The `edition` field is used to detect config compatibility. If your config's edition differs from the current one, Archie will warn you on startup but continue with your settings.

### Fields
| Field | Required | Description |
|---|---|---|
| `key` | yes | Single-character trigger |
| `action` | yes | Command to run |
| `desc` | no | Help text |
| `prompt` | no | Prompts for input, fills `{placeholder}` in action |
| `confirm` | no | Requires y/N before executing |

### Action prefixes
| Prefix | Behavior |
|---|---|
| *(none)* | Executed directly |
| `shell:` | Passed to `sh -c` — use for pipes and redirects |
| `builtin:quit` | Exit Archie |
| `builtin:help` | Show help |

### Custom commands
```toml
[commands]
mirror = { key = "m", action = "shell:rate-mirrors --save /etc/pacman.d/mirrorlist arch", desc = "update mirrorlist" }
diff   = { key = "d", action = "shell:pacdiff",                                           desc = "review pacdiff files" }
```
If it runs in a shell, Archie can run it.

## Package cache
Tab completion is backed by two databases in `~/.cache/archie/`:

| File | Source | Used for |
|---|---|---|
| `available.db` | `paru -Slq` | install, search prompts |
| `installed.db` | `paru -Qq` | remove, purge prompts |

Both are populated on first launch if absent. After any install, remove, or update action they are refreshed in the background, so the next session always has a current list.

## License
GPL-3.0 — see [LICENSE](LICENSE).

*Not affiliated with Arch Linux or paru.*
