# Archie

> Even faster & easier package management for Arch Linux.

A minimal interactive wrapper for `paru`. An active rewrite of TuxForge/archie in Rust.

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

## Why rewrite in Rust?

### Memory leaks

`strdup()` is called in `get_package_manager_version()`, `command_generator()`, and `get_pacman_commands()` with no corresponding `free()`. Every tab completion, every version check, every package list fetch leaks memory.

```c
char* get_package_manager_version(const char *package_manager) {
    return strdup(version_start); // never freed
}
commands[command_count - 1] = strdup(path); // array never freed
```

**Rust fix:** Ownership model. If it compiles, it doesn't leak.

---

### Shell injection

Every operation builds a command string and passes it to `system()`. No input validation.

```c
snprintf(command, sizeof(command), "%s -S %s", package_manager, package);
system(command); // package = "foo; rm -rf ~" works fine
```

**Rust fix:** `Command::new()` with explicit arg arrays. No shell, no injection.

---

### Buffer overflow in `get_input()`

```c
#define MAX_INPUT_LENGTH 256
strcpy(input, line); // line can be longer than 256 bytes → stack corruption
```

**Rust fix:** `String` grows dynamically. No fixed buffers, no overflows.

---

### Stack overflow on empty input

```c
void get_input(...) {
    if (strlen(input) == 0) {
        get_input(input, prompt); // recurse forever on Enter spam
    }
}
```

**Rust fix:** Loops, not recursion. Stack stays happy.

---

### Unbounded `scanf`

```c
char response[10];
scanf("%s", response); // input >9 chars → buffer overflow
```

**Rust fix:** `readline()` returns a `String`. No width limits needed.

---

### Global mutable state in completion

```c
static char **commands = NULL; // fetched once, leaked forever, never refreshed
```

**Rust fix:** Packages loaded once at startup, owned by the completer, dropped on exit. Refresh by restarting (or add a `refresh` command later).

---

### Tab completion blocks 2–5 seconds

`pacman -Ssq` runs synchronously on first Tab. Terminal freezes.

**Rust fix:** Load packages at startup (~100ms). Completion is instant thereafter.

---

### Wasteful `system()` calls for binary checks

```c
system("command -v yay > /dev/null 2>&1"); // forks a shell just to check PATH
```

**Rust fix:** We assume `paru` is installed (it's a dependency). No runtime checks needed.

---

### Fragile shell compound commands

```c
system("mkdir -p $HOME/... && cd ... && git clone ... && makepkg ...");
// Fails silently if $HOME has spaces, or any step fails midway
```

**Rust fix:** Not applicable — `archie` doesn't install package managers. Let the user manage their setup.

---

### Summary

| Issue | C v1.3 | Rust v3.1+ |
|-------|--------|------------|
| Shell injection | ✅ Possible | ❌ Prevented |
| Memory leaks | ✅ Present | ❌ Eliminated |
| Buffer overflow | ✅ Possible | ❌ Eliminated |
| Stack overflow | ✅ Possible | ❌ Eliminated |
| Unbounded `scanf` | ✅ Present | ❌ Eliminated |
| Global mutable state | ✅ Present | ❌ Eliminated |
| Completion latency | ~3000ms | ~0ms (after load) |
| Startup time | ~3.4ms | ~0.8ms |
| Build | `make` + readline | `cargo build` |
| Binary size | ~50KB | ~3.5MB (static, worth it) |

The rewrite isn't about features. It's about sleeping at night knowing your package wrapper won't segfault, leak, or get pwned by a cleverly named package.

## License

GPL-3.0 — see [LICENSE](LICENSE) for details.

---

*Archie is not affiliated with Arch Linux, paru, or the Arch Wiki.*
