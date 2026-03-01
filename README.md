# Archie

> Even faster & easier package management for Arch Linux.

A minimal interactive wrapper for `paru`. An active rewrite of TuxForge/archie in Rust.

<details>
<summary><strong>Why rewrite in Rust?</strong></summary>

The C version worked, but had critical issues:

| Issue | C v1.3 | Rust v3.2+ |
|-------|--------|------------|
| Shell injection via `system()` | ✅ Possible | ❌ Prevented |
| Memory leaks (`strdup` w/o `free`) | ✅ Present | ❌ Eliminated |
| Buffer/stack overflows | ✅ Possible | ❌ Eliminated |
| Blocking tab completion (~3s) | ✅ Yes | ❌ Instant |
| Build complexity | `make` + readline | `cargo build` |

**Bottom line:** Same UX, zero footguns. The rewrite is about safety, not features.

</details>

## Features

- Single-letter commands for common operations
- Tab completion for all official + AUR packages
- `-e` / `--exec` mode for scripting
- Zero runtime dependencies beyond `paru`
- Memory-safe, injection-proof, instant startup

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

### Interactive mode

```
$ archie

Welcome to Archie v3.1.1
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

Tab completion works automatically when entering package names.

### Exec mode (scripting)

```bash
archie -e u                # Update system
archie -e i firefox        # Install firefox (no prompt)
archie -e i                # Prompt for package (with completion)
archie -e s                # Prompt for search query
archie -e o                # Remove orphans
archie --exec h            # --exec also works

# Exit codes: 0 = success, 1 = invalid command
```

### Version

```bash
archie --version
```

```
    __     
 .:--.'.   Archie-ng v3.2.0 - Fast & Easy package management for Arch Linux
/ |   \ |  Written in Rust, powered by paru.
`" __ | |  paru v2.1.0.r67.g9ac3578 +git - libalpm v16.0.1
 .'.''| |  
/ /   | |_ This program may be freely redistributed under the terms of the GNU General Public License.
\ \._,\ '/ Created & maintained by Gurov
 `--'  `"  
```

## Tab Completion Setup

Completion works out of the box. To keep AUR packages up to date:

```bash
# Generate/update the AUR package cache (run occasionally)
paru -Pc
```

Official repo packages are loaded automatically via `pacman -Slq`.

## License

GPL-3.0 — see [LICENSE](LICENSE) for details.

---

*Archie is not affiliated with Arch Linux, paru, or the Arch Wiki.*
