# Archie

> Even faster & easier package management for Arch Linux.

Archie is a small interactive wrapper around `paru`. It keeps the usual workflow, just trims the fat. Short keys, quick commands, no waiting around.

This is a ground-up rewrite of the original TuxForge/archie in Rust, focused on safety and responsiveness rather than piling on features.

It does one job. It does it fast.

## Why the rewrite?

The old C version worked, but it had sharp edges. The kind you only notice at 2am when something breaks.

| Problem                        | C v1.3          | Rust v3.4+  |
| ------------------------------ | --------------- | ----------- |
| Shell injection via `system()` | Possible        | Prevented   |
| Memory leaks                   | Present         | Gone        |
| Buffer/stack overflows         | Possible        | Gone        |
| Tab completion lag (~3s)       | Yes             | Instant     |
| Build setup                    | make + readline | cargo build |

In short: fewer footguns, better behavior, less babysitting.

## Features

* Single-letter commands for common tasks
* Tab completion for official + AUR packages
* `--exec` mode for scripting
* Configurable commands via `~/.config/archie/archie.toml`
* No runtime deps besides `paru`
* Fast startup, memory-safe, no shell surprises

Nothing fancy. Just quick.

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
sudo install -Dm755 target/release/archie /usr/local/bin/archie
```

**Prerequisites**

* Rust toolchain (`rustup`)
* `paru`

## Usage

### Interactive mode

```text
❯ archie

Archie v3.5.0-rc1 — type h for help
❯ h

  key   description
  ──────────────────────────────────
  c     clean package cache
  h     show this help
  i     install a package
  l     view recent pacman log
  o     remove orphaned packages
  p     remove package + deps
  q     exit archie
  r     remove a package
  s     search packages
  u     upgrade all packages
```

Type a key. Done. Tab completion handles package names automatically.

### Exec mode (scripting)

For scripts or aliases, skip the interactive UI:

```bash
archie -e update
archie -e install firefox
archie -e install        # prompts with completion
archie -e search
archie -e orphans
archie --exec help
```

**Exit codes**

* `0` success
* `1` invalid command

### Version

```bash
archie --version
```

## Configuration

Archie reads:

```
~/.config/archie/archie.toml
```

If it doesn’t exist, it creates one with sensible defaults.

Every command is user-defined. Change keys, swap actions, or add your own.

### Default config

```toml
[commands]
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
```

### Fields

| Field     | Required | Meaning                                  |
| --------- | -------- | ---------------------------------------- |
| `key`     | yes      | single character trigger                 |
| `action`  | yes      | command to execute                       |
| `desc`    | no       | help text                                |
| `prompt`  | no       | asks for input and fills `{placeholder}` |
| `confirm` | no       | y/N confirmation                         |

### Action types

| Prefix         | Behavior            |
| -------------- | ------------------- |
| none           | executed directly   |
| `shell:`       | run through `sh -c` |
| `builtin:quit` | exit Archie         |
| `builtin:help` | show help           |

Use `shell:` only when you need pipes or redirects.

### Custom commands

Add whatever fits your workflow:

```toml
[commands]
mirror = { key = "m", action = "shell:rate-mirrors --save /etc/pacman.d/mirrorlist arch", desc = "update mirrorlist" }
diff   = { key = "d", action = "shell:pacdiff",                                           desc = "review pacdiff files" }
notes  = { key = "n", action = "shell:bat ~/.local/share/archie/notes.log",               desc = "view notes" }
```

If it runs in your shell, Archie can run it.

## Tab completion

Works out of the box.

To refresh AUR package data:

```bash
paru -Pc
```

Official packages are read automatically from `pacman`.

## License

GPL-3.0. See `LICENSE` for details.

*Archie is not affiliated with Arch Linux or paru.*
