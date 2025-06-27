# Archie-ng

Even faster & easier package management for Arch Linux.

## What it does

- Update system packages
- Install/remove packages  
- Search repositories
- Clean cache and orphans
- Works with paru, yay, or pacman

## Install

```bash
git clone https://github.com/Gur0v/archie-ng.git
cd archie-ng
make
sudo make install
```

## Use

Run `archie` for interactive mode:

```
archie> u    # update system
archie> i    # install package
archie> s    # search packages
archie> q    # quit
```

Or run commands directly:
```bash
archie --exec u
```

## Commands

- `u` - Update system
- `i` - Install package
- `r` - Remove package  
- `p` - Purge package + deps
- `s` - Search packages
- `c` - Clean cache
- `o` - Remove orphans
- `h` - Help
- `q` - Quit

## Why rewrite?

The original had security issues and messy code. This version:
- Validates all input
- Uses safe string handling
- Cleaner architecture
- No config files needed

## Requirements

- GCC
- Git (installed automatically if missing)
- At least one of: paru, yay, pacman

---

Made by Gurov. GPL-3.0 license.
