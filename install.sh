#!/usr/bin/env bash

set -euo pipefail

repo="herotomg/toms-tools"
bin_name="tt"
install_dir="${TT_INSTALL_DIR:-$HOME/.local/bin}"
install_path="$install_dir/$bin_name"
path_block_start="# >>> tt PATH setup >>>"
path_block_end="# <<< tt PATH setup <<<"

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
    shell_name="$(basename "${SHELL:-}")"
    rc_file=""
    path_block=""

    case "$shell_name" in
      bash)
        if [ -f "$HOME/.bashrc" ]; then
          rc_file="$HOME/.bashrc"
        elif [ -f "$HOME/.bash_profile" ]; then
          rc_file="$HOME/.bash_profile"
        elif [ -f "$HOME/.profile" ]; then
          rc_file="$HOME/.profile"
        else
          rc_file="$HOME/.bashrc"
        fi
        path_block=$(cat <<EOF
$path_block_start
export PATH="$install_dir:\$PATH"
$path_block_end
EOF
)
        ;;
      zsh)
        rc_file="$HOME/.zshrc"
        path_block=$(cat <<EOF
$path_block_start
export PATH="$install_dir:\$PATH"
$path_block_end
EOF
)
        ;;
      fish)
        rc_file="$HOME/.config/fish/config.fish"
        path_block=$(cat <<EOF
$path_block_start
if not contains -- "$install_dir" \$PATH
    fish_add_path -g "$install_dir"
end
$path_block_end
EOF
)
        ;;
    esac

    if [ -n "$rc_file" ]; then
      mkdir -p "$(dirname "$rc_file")"
      touch "$rc_file"

      tmp_rc="$(mktemp)"
      awk -v start="$path_block_start" -v end="$path_block_end" '
        $0 == start { in_block = 1; next }
        $0 == end { in_block = 0; next }
        !in_block { print }
      ' "$rc_file" > "$tmp_rc"
      mv "$tmp_rc" "$rc_file"

      if [ -s "$rc_file" ]; then
        printf '\n' >> "$rc_file"
      fi
      printf '%s\n' "$path_block" >> "$rc_file"

      echo "Added $install_dir to PATH in $rc_file"
      echo "Open a new shell or run: source $rc_file"
    else
      echo "Add $install_dir to your PATH to run $bin_name from anywhere."
      echo "For example: export PATH=\"$install_dir:\$PATH\""
    fi
    ;;
esac