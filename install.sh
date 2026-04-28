#!/usr/bin/env bash

set -euo pipefail

repo="herotomg/toms-tools"
bin_name="tt"
install_dir="${TT_INSTALL_DIR:-$HOME/.local/bin}"
install_path="$install_dir/$bin_name"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Darwin) os_part="apple-darwin" ;;
  Linux) os_part="unknown-linux-gnu" ;;
  *)
    echo "Unsupported operating system: $os" >&2
    exit 1
    ;;
esac

case "$arch" in
  x86_64) arch_part="x86_64" ;;
  arm64|aarch64) arch_part="aarch64" ;;
  *)
    echo "Unsupported architecture: $arch" >&2
    exit 1
    ;;
esac

target="$arch_part-$os_part"
archive="tt-$target.tar.gz"
url="https://github.com/$repo/releases/latest/download/$archive"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

mkdir -p "$install_dir"

echo "Downloading $url"
curl -fsSL "$url" -o "$tmpdir/$archive"
tar -xzf "$tmpdir/$archive" -C "$tmpdir"

if [ ! -f "$tmpdir/$bin_name" ]; then
  echo "Downloaded archive did not contain $bin_name" >&2
  exit 1
fi

cp "$tmpdir/$bin_name" "$install_path"
chmod +x "$install_path"

echo "Installed $bin_name to $install_path"

case ":${PATH:-}:" in
  *":$install_dir:"*) ;;
  *)
    echo "Add $install_dir to your PATH to run $bin_name from anywhere."
    echo "For example: export PATH=\"$install_dir:\$PATH\""
    ;;
esac