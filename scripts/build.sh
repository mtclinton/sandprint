#!/usr/bin/env bash
# Wrapper around `cargo build` that redirects CARGO_TARGET_DIR away from
# the source tree if the project path contains a space. libbpf-sys's
# Makefile-driven static build does not quote paths and breaks on
# whitespace, so the workaround is to put the build directory somewhere
# the path is whitespace-free.

set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "$PWD" == *" "* ]]; then
    : "${CARGO_TARGET_DIR:=${TMPDIR:-/tmp}/sandprint-target}"
    export CARGO_TARGET_DIR
    mkdir -p "$CARGO_TARGET_DIR"
    echo "note: project path contains whitespace; using CARGO_TARGET_DIR=$CARGO_TARGET_DIR" >&2
fi

exec cargo build "$@"
