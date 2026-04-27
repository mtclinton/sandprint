#!/usr/bin/env bash
#
# Profile a curl invocation and print an OCI seccomp profile.
#
# Usage: ./profile-curl.sh [URL]
# Env:   SP=/path/to/sandprint  (defaults to `sandprint` in PATH)

set -euo pipefail

SP=${SP:-sandprint}
URL=${1:-https://example.com}
TRACE=$(mktemp /tmp/sandprint-curl.XXXXXX.json)
trap 'rm -f "$TRACE"' EXIT

"$SP" profile run --output "$TRACE" -- \
    curl -s -o /dev/null "$URL"

"$SP" profile generate --input "$TRACE" --format oci
