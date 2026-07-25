# Downloading

PREREQUISITES (non-Windows):
- [git](https://git-scm.com/install)
- [Rust](https://rust-lang.org/tools/install/)

## Linux and macOS

If you're on Arch, or another Arch based distro, install off the AUR with [yay](https://github.com/jguer/yay):
```bash
yay -S buf-cli
```
Prefer `paru`? Run `paru -S buf-cli` instead.

**If you're on another distribution or macOS**:
```bash
curl -fsSL https://raw.githubusercontent.com/brysonak/buf/refs/heads/main/Install/install.sh | sh
```
**NOTE**: This script will ask for privileges, and needs a C compiler (`gcc`/`clang`) on PATH to link the rust binary. **If you're on NixOS**, use the flake instead, see below.

## NixOS

NixOS users can install straight from the flake.
```bash
nix profile install github:brysonak/buf
```

## Windows

Run the `buf-setup.exe` installer from the [releases page](https://github.com/brysonak/buf/releases).
