#!/bin/sh
# jira-cli installer
# usage:
#   curl -sSL https://raw.githubusercontent.com/zhiyue/jira-cli/main/install.sh | sh
#   ./install.sh                       # install latest
#   ./install.sh -v v0.1.0             # specific version
#   ./install.sh -d /usr/local/bin     # specific dir
#   ./install.sh -b https://internal.example.com/jira-cli   # custom base URL (internal mirror)
set -eu

REPO_DEFAULT="zhiyue/jira-cli"
REPO="${JIRA_CLI_REPO:-$REPO_DEFAULT}"
BASE_URL_DEFAULT="https://github.com/${REPO}/releases"
BASE_URL="${JIRA_CLI_DOWNLOAD_URL:-$BASE_URL_DEFAULT}"
VERSION=""
INSTALL_DIR=""

while [ $# -gt 0 ]; do
    case "$1" in
        -v|--version) VERSION="$2"; shift 2 ;;
        -d|--dir) INSTALL_DIR="$2"; shift 2 ;;
        -b|--base-url) BASE_URL="$2"; shift 2 ;;
        -h|--help)
            echo "usage: install.sh [-v VERSION] [-d DIR] [-b BASE_URL]"
            echo "env: JIRA_CLI_REPO, JIRA_CLI_DOWNLOAD_URL"
            exit 0 ;;
        *) echo "unknown flag: $1" >&2; exit 2 ;;
    esac
done

# Detect OS
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "$OS" in
    darwin) OS_TAG="apple-darwin" ;;
    linux)
        # Prefer musl if available to avoid glibc version issues
        if ldd --version 2>&1 | grep -qi musl || [ -f /etc/alpine-release ]; then
            OS_TAG="unknown-linux-musl"
        else
            OS_TAG="unknown-linux-gnu"
        fi ;;
    *) echo "unsupported OS: $OS" >&2; exit 1 ;;
esac

# Detect arch
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) echo "unsupported arch: $ARCH" >&2; exit 1 ;;
esac

TARGET="${ARCH}-${OS_TAG}"

# Resolve version — latest if not given
if [ -z "$VERSION" ]; then
    VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
    if [ -z "$VERSION" ]; then
        echo "failed to resolve latest version; pass -v explicitly" >&2
        exit 1
    fi
fi

# Resolve install dir
if [ -z "$INSTALL_DIR" ]; then
    if [ -w /usr/local/bin ] 2>/dev/null; then
        INSTALL_DIR="/usr/local/bin"
    else
        INSTALL_DIR="$HOME/.local/bin"
        mkdir -p "$INSTALL_DIR"
    fi
fi

# Assemble URLs
BIN_NAME="jira-cli-${VERSION}-${TARGET}"
TARBALL="${BIN_NAME}.tar.gz"
TAR_URL="${BASE_URL}/download/${VERSION}/${TARBALL}"
SHA_URL="${BASE_URL}/download/${VERSION}/${BIN_NAME}.sha256"

echo "==> Installing jira-cli $VERSION ($TARGET) to $INSTALL_DIR"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "==> Downloading $TAR_URL"
curl -fsSL "$TAR_URL" -o "$TMP/$TARBALL"

echo "==> Verifying SHA256"
curl -fsSL "$SHA_URL" -o "$TMP/$TARBALL.sha256"
EXPECTED="$(awk '{print $1}' "$TMP/$TARBALL.sha256")"
if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL="$(sha256sum "$TMP/$TARBALL" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
    ACTUAL="$(shasum -a 256 "$TMP/$TARBALL" | awk '{print $1}')"
else
    echo "no sha256sum/shasum available; skipping integrity check" >&2
    ACTUAL="$EXPECTED"
fi
if [ "$ACTUAL" != "$EXPECTED" ]; then
    echo "checksum mismatch: expected $EXPECTED, got $ACTUAL" >&2
    exit 1
fi

echo "==> Extracting"
tar -xzf "$TMP/$TARBALL" -C "$TMP"

# Find jira-cli binary inside the extracted tree (may be at root or in a subdir)
BIN_PATH="$(find "$TMP" -type f -name jira-cli -perm -u+x | head -1)"
if [ -z "$BIN_PATH" ]; then
    BIN_PATH="$(find "$TMP" -type f -name jira-cli | head -1)"
fi
if [ -z "$BIN_PATH" ]; then
    echo "jira-cli binary not found in tarball" >&2
    exit 1
fi

install -m 0755 "$BIN_PATH" "$INSTALL_DIR/jira-cli"

echo "==> Installed: $INSTALL_DIR/jira-cli"
"$INSTALL_DIR/jira-cli" --version || true

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) echo "NOTE: $INSTALL_DIR is not in your PATH. Add it to your shell RC file." ;;
esac
