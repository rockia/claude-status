#!/bin/sh
set -e

REPO="rockia/csl"
INSTALL_DIR="$HOME/.claude"

detect_platform() {
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Darwin) os_name="macos" ;;
    Linux)  os_name="linux" ;;
    *)
      echo "Error: unsupported OS: $os" >&2
      exit 1
      ;;
  esac

  case "$arch" in
    x86_64|amd64)   arch_name="x86_64" ;;
    arm64|aarch64)   arch_name="aarch64" ;;
    *)
      echo "Error: unsupported architecture: $arch" >&2
      exit 1
      ;;
  esac

  echo "csl-${arch_name}-${os_name}"
}

main() {
  artifact="$(detect_platform)"
  echo "Detected platform: $artifact"

  # Get latest release tag
  tag="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)"
  if [ -z "$tag" ]; then
    echo "Error: could not determine latest release" >&2
    exit 1
  fi
  echo "Latest release: $tag"

  url="https://github.com/${REPO}/releases/download/${tag}/${artifact}.tar.gz"
  echo "Downloading $url"

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  curl -fsSL "$url" -o "$tmpdir/${artifact}.tar.gz"
  tar xzf "$tmpdir/${artifact}.tar.gz" -C "$tmpdir"

  mkdir -p "$INSTALL_DIR"
  mv "$tmpdir/csl" "$INSTALL_DIR/csl"
  chmod +x "$INSTALL_DIR/csl"

  echo "Installed csl to $INSTALL_DIR/csl"
  echo "Configuring Claude Code statusline..."
  "$INSTALL_DIR/csl" install

  echo "Done! Restart Claude Code to see your new statusline."
}

main
