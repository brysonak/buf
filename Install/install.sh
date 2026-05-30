#!/bin/sh
REPO="https://github.com/brysonak/buf.git"
BIN="buf"

check_dep() {
    if ! command -v "$1" > /dev/null 2>&1; then
        echo "$1 is needed to run this, please install it before running this script again."
        exit 1
    fi
}

OS=$(uname -s)

case "$OS" in
    Darwin)
        INSTALL_DIR="/usr/local/bin"
        check_dep git
        check_dep cargo
        check_dep clang
        ;;
    Linux)
        INSTALL_DIR="/usr/bin"
        check_dep git
        check_dep cargo
        check_dep cc
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

CLONE_DIR=$(mktemp -d)
git clone "$REPO" "$CLONE_DIR"
cd "$CLONE_DIR"
cargo build --release
sudo cp "target/release/$BIN" "$INSTALL_DIR/$BIN"
echo "buf installed to $INSTALL_DIR/$BIN, restart your shell to use it"
rm -rf "$CLONE_DIR"