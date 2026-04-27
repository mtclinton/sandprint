#!/usr/bin/env bash
# Install build prerequisites on Debian/Ubuntu and Fedora-family hosts.
# This is a convenience wrapper, not a substitute for reading the
# README's prerequisites section.

set -euo pipefail

if [[ -f /etc/debian_version ]]; then
    sudo apt-get update
    sudo apt-get install -y \
        clang \
        llvm \
        libelf-dev \
        zlib1g-dev \
        linux-libc-dev \
        pkg-config \
        make
elif [[ -f /etc/fedora-release ]] || [[ -f /etc/redhat-release ]]; then
    sudo dnf install -y \
        clang \
        llvm \
        elfutils-libelf-devel \
        zlib-devel \
        kernel-headers \
        pkgconf \
        make
else
    echo "Unsupported distribution. Install clang, libelf-dev, zlib-dev," >&2
    echo "kernel headers, and pkg-config manually." >&2
    exit 1
fi

if ! command -v rustup >/dev/null 2>&1; then
    echo
    echo "rustup is not installed. Install it from https://rustup.rs and re-run." >&2
fi
