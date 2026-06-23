#!/usr/bin/env bash
# Parse-check and ASCII-preview a tte scene or model via the headless CLI.
# Errors carry 1-based line numbers; a clean run prints one rendered frame.
#
# Usage: .claude/skills/tte-scene/validate.sh <file.scene|file.obj> [WxH]
#   WxH defaults to 80x40.
set -euo pipefail

file="${1:?usage: validate.sh <file.scene|file.obj> [WxH]}"
size="${2:-80x40}"

# Repo root, regardless of where this is invoked from.
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

# Run from the repo workspace via --manifest-path so the file path stays relative
# to the caller's CWD.
exec cargo run -q --manifest-path "$root/Cargo.toml" -p tte-cli -- \
  view --headless --frames 1 --size "$size" \
  --render solid --shading gouraud --mode ascii "$file"
