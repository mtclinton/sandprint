#!/usr/bin/env bash
#
# Compare the syscall sets of two different ways of running the same
# command. Helpful for spotting which runtime needs a wider seccomp
# allowlist before deciding which one to sandbox.
#
# Usage: ./compare-runtimes.sh
# Env:   SP=/path/to/sandprint  (defaults to `sandprint` in PATH)
#
# This script runs curl natively and again under `docker run`. Both
# invocations need to succeed for the diff to be meaningful; if your
# host doesn't have docker installed, swap the second invocation for
# `podman`, `runc`, or whatever runtime you want to compare.

set -euo pipefail

SP=${SP:-sandprint}
URL=${URL:-https://example.com}

NATIVE=$(mktemp /tmp/sandprint-native.XXXXXX.json)
RUNTIME=$(mktemp /tmp/sandprint-docker.XXXXXX.json)
trap 'rm -f "$NATIVE" "$RUNTIME"' EXIT

"$SP" profile run --output "$NATIVE" -- \
    curl -s -o /dev/null "$URL"

"$SP" profile run --output "$RUNTIME" -- \
    docker run --rm curlimages/curl -s -o /dev/null "$URL"

echo "=== diff: native vs docker ==="
"$SP" profile diff "$NATIVE" "$RUNTIME"
