# Archie

> Even faster & easier package management for Arch Linux.

A minimal interactive wrapper for `paru`. An active rewrite of TuxForge/archie in Rust.

## Features

- Single-letter commands for common operations
- No external dependencies

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

Welcome to Archie v3.x.x
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

<details>
<summary>Why rewrite in Rust?</summary>

### Memory leaks

`strdup()` is called in `get_package_manager_version()`, `command_generator()`, and `get_pacman_commands()` with no corresponding `free()`. Every tab completion, every version check, every package list fetch leaks memory. The program is short-lived enough that the OS cleans up on exit, but it's still wrong.

```c
char* get_package_manager_version(const char *package_manager) {
    // ...
    return strdup(version_start); // never freed by the caller
}

commands[command_count - 1] = strdup(path); // array never freed
return strdup(commands[list_index++]);       // completion matches never freed
```

### Shell injection

Every package operation builds a command string and passes it to `system()`, which invokes a shell. There is no input validation anywhere. A package name containing shell metacharacters executes arbitrary commands as the current user.

```c
snprintf(command, sizeof(command), "%s -S %s", package_manager, package);
system(command); // package = "foo; rm -rf ~" works fine here
```

This applies to install, remove, purge, and search — every operation that takes user input.

### Buffer overflow in `get_input()`

`readline()` returns a heap-allocated string of arbitrary length. It gets copied into a fixed 256-byte buffer with `strcpy()`, no bounds checking.

```c
#define MAX_INPUT_LENGTH 256
char input[MAX_INPUT_LENGTH];

char *line = readline(prompt);
strcpy(input, line); // line can be longer than 256 bytes
```

Any input over 255 characters corrupts the stack.

### Stack overflow in `get_input()`

Empty input causes `get_input()` to call itself recursively with no base case other than the user eventually typing something. There is no iteration limit.

```c
void get_input(char *input, const char *prompt) {
    // ...
    if (strlen(input) == 0) {
        get_input(input, prompt); // recurse forever on empty input
    }
}
```

The same recursive pattern exists in `handle_command()` for the "did you mean?" flow. Holding Enter will segfault the process.

### Recursive "did you mean?" flow

If a user types something like `iq` (starts with a valid command but has extra characters), `handle_command()` prompts "did you mean `i`?" and if confirmed, calls `handle_command()` again recursively. That second call can trigger the same branch again.

```c
void handle_command(const char *input, const char *package_manager) {
    // ...
    if (is_valid_command(choice)) {
        // prompt user...
        handle_command(new_input, package_manager); // recursive call
    }
}
```

It's one level deep in practice, but it's still unnecessary recursion where a simple loop works fine.

### Unbounded `scanf` in `prompt_install_yay()`

```c
char response[10];
scanf("%s", response); // no width limit, overflows response[10]
```

Any input over 9 characters overflows the buffer. `scanf("%9s", response)` would have been the fix, but this entire function was unnecessary to begin with.

### Global mutable state in tab completion

The readline completion callback uses static local variables to maintain state across calls. This is a C idiom, but it's not re-entrant and not thread-safe. More practically, the package list is fetched once and cached in a static pointer that's never freed and never refreshed.

```c
char *command_generator(const char *text, int state) {
    static int list_index, len;
    static char **commands = NULL;

    if (!commands) {
        commands = get_pacman_commands(); // fetched once, leaked forever
    }
    // ...
}
```

If the package list changes during a session, the completions go stale with no way to refresh short of restarting the program.

### Tab completion blocks for 2–5 seconds

`get_pacman_commands()` runs `pacman -Ssq` synchronously and reads the entire output before returning. This happens on the first Tab press and takes 2–5 seconds depending on the system. There's no progress indicator, no async fetch, nothing — the terminal just freezes.

```c
fp = popen("pacman -Ssq", "r");
while (fgets(path, sizeof(path), fp) != NULL) {
    // read thousands of lines synchronously
}
```

### `check_package_manager()` calls `system()` unnecessarily

```c
int check_package_manager() {
    if (check_archie_file()) return 2;
    if (system("command -v yay > /dev/null 2>&1") == 0) return 1;
    if (system("command -v paru > /dev/null 2>&1") == 0) return 2;
    return 0;
}
```

Spawning a shell just to check if a binary exists is wasteful. `access("/usr/bin/yay", X_OK)` or `stat()` would do the same thing without forking a shell process.

### `install_yay()` passes a compound shell command to `system()`

```c
system("mkdir -p $HOME/.cache/archie/made-by-gurov && "
       "cd $HOME/.cache/archie/made-by-gurov && "
       "git clone https://aur.archlinux.org/yay-bin.git && "
       "cd yay-bin && "
       "makepkg -scCi && "
       "cd && "
       "rm -rf $HOME/.cache/archie/");
```

This relies entirely on shell expansion of `$HOME`. If the shell isn't bash or `$HOME` contains spaces or special characters, this silently misbehaves. Each `&&` chain also means a failure partway through leaves a partial install with no cleanup or error reporting.

### Summary

| Issue | C v1.3 | Rust v3.0+ |
|---|---|---|
| Shell injection | possible | prevented |
| Memory leaks | present | eliminated |
| Buffer overflow on long input | possible | eliminated |
| Stack overflow on empty input | possible | eliminated |
| Unbounded `scanf` | present | eliminated |
| Global mutable state | present | eliminated |
| Startup time | ~3.4ms | ~0.8ms |
| Build | `make` + readline dep | `cargo build` |

</details>

## License

GPL-3.0 — see [LICENSE](LICENSE) for details.

---

*Not affiliated with Arch Linux, paru, or the Arch Wiki.*
