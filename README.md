# Archie-ng

Even faster & easier package management for Arch Linux.

## What it does

Archie-ng is a modern, secure wrapper around Arch Linux package managers that provides:

- **System Updates** - Keep your system up-to-date effortlessly
- **Package Management** - Install, remove, and purge packages with ease
- **Smart Search** - Find packages across repositories instantly
- **Cache Management** - Clean package cache and remove orphaned packages
- **Auto-detection** - Automatically works with paru, yay, or pacman
- **Tab Completion** - Smart package name completion for faster workflow
- **Security First** - Input validation and safe command execution

## Installation

### Prerequisites
- A C compiler (GCC, Clang, etc.)
- Make
- Readline library (`readline` package)
- Git (will be installed automatically if missing)
- At least one package manager: paru, yay, or pacman

### Quick Install
```bash
git clone --depth=1 https://github.com/Gur0v/archie-ng
cd archie-ng
sudo make clean install
```

## Usage

### Interactive Mode
Simply run `archie-ng` to enter interactive mode:

```
$ archie-ng

Welcome to Archie-ng v2.1.0
Using paru package manager
Type 'h' for help

$ u    # Update system
$ i    # Install package (with tab completion!)
$ s    # Search packages
$ q    # Quit
```

### Command Line Arguments
```bash
archie-ng --version    # Show version
archie-ng --help       # Show help
```

## Commands Reference

| Command | Action | Description |
|---------|--------|-------------|
| `u` | Update | Update all system packages |
| `i` | Install | Install a package (with tab completion) |
| `r` | Remove | Remove a package |
| `p` | Purge | Remove package and its dependencies |
| `s` | Search | Search for packages in repositories |
| `c` | Clean | Clean package cache |
| `o` | Orphans | Remove orphaned packages |
| `h` | Help | Show available commands |
| `q` | Quit | Exit the program |

## Why the Rewrite?

The original Archie (v1.3) worked but had several limitations that made a rewrite necessary:

### What Changed:

**🏗️ Architecture Overhaul**
- **Old**: Single large file with mixed concerns and global state
- **New**: Modular design with clear separation between package detection, command handling, and user interface

**🔧 Package Manager Detection**
- **Old**: Simple binary existence checks with hardcoded preference for paru via config file
- **New**: Enum-based system with structured detection logic and automatic fallback

**💻 Memory Management**
- **Old**: Mixed malloc/free with potential leaks in completion system
- **New**: Consistent memory handling with proper cleanup functions

**⌨️ Input Handling**
- **Old**: Basic readline integration with simple command parsing
- **New**: Enhanced readline with history management, better completion caching, and input validation

**🛡️ Security Improvements**
- **Old**: Direct string concatenation in command building
- **New**: Input validation with `is_valid_package_name()` to prevent injection attacks

**📦 Completion System**
- **Old**: Rebuilt package list on every completion attempt
- **New**: Efficient caching system that reuses completion data and proper cleanup

**🎯 Code Quality**
- **Old**: 400+ lines in a single function-heavy file
- **New**: Organized into logical sections with clear function responsibilities and better error handling

The rewrite focused on maintainability, security, and performance while keeping the same simple user interface that made the original popular.

## License

GPL-3.0 License - see [LICENSE](LICENSE) file for details.

---

*Disclaimer: Archie-ng **is not affiliated** with Arch Linux or its official package managers.*
