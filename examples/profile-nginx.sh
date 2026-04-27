#!/usr/bin/env bash
#
# Attach to a running nginx and emit a systemd SystemCallFilter line
# from the syscalls it makes during the trace window.
#
# Usage: ./profile-nginx.sh [DURATION_SECS]
# Env:   SP=/path/to/sandprint  (defaults to `sandprint` in PATH)

set -euo pipefail

SP=${SP:-sandprint}
DURATION=${1:-30}

PID=$(pgrep -o nginx || true)
if [[ -z "$PID" ]]; then
    echo "no nginx process found" >&2
    exit 1
fi

TRACE=$(mktemp /tmp/sandprint-nginx.XXXXXX.json)
trap 'rm -f "$TRACE"' EXIT

"$SP" profile attach --pid "$PID" --duration "$DURATION" --output "$TRACE"
"$SP" profile generate --input "$TRACE" --format systemd
