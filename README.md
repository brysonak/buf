# buf
![buf logo](buf.png)
**B**ootable **U**SB **F**lasher is a tool made for flashing .iso/.img files onto USB drives, for booting into operating systems of course...
 Is it bootable USB flasher or Bryson's USB flasher?

buf is fully cross-platform and open source, under [GPL-v3](https://www.gnu.org/licenses/gpl-3.0.html).

Logo made by [Mia](https://github.com/marshmallow-mia)

# Installation 
PREREQUISITES:
- [git](https://git-scm.com/install)
- [Rust](https://rust-lang.org/tools/install/)

## Linux and macOS
If you're on Arch, or another Arch Linux based distro, you can install off the AUR with [yay](https://github.com/jguer/yay):
```bash
yay -S buf-cli
```
If you prefer to use `paru` instead, run ``paru -S buf-cli``

Run the following command:
```bash
curl -fsSL https://raw.githubusercontent.com/brysonak/buf/refs/heads/main/Install/install.sh | sh
```
**NOTE**: This script will ask for priviledges.

## Windows
Run the `buf-setup.exe` installer from the [releases page](https://github.com/brysonak/buf/releases).


# Building
Pre-requisites:
- [git](https://git-scm.com/install)
- [Rust](https://rust-lang.org/tools/install/)

Run the following commands:
```bash
git clone https://github.com/brysonak/buf.git

cd buf

cargo build --release
```
This will produce a binary inside `target/release`.

To use it straight from the CLI, it's recommended that you add it to PATH on windows. If you're on linux, copy the binary into /usr/bin (or /usr/local/bin) then restart any open shell to use it.