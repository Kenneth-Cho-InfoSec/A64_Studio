#!/usr/bin/env bash
# Run the Rust core's tests under QEMU user-mode emulation.
#
# This is a CI/desktop fallback, not a substitute for a physical ARM64 device.
# It validates the AArch64 code paths (including the execution trampoline) when
# the host is x86_64. Android-specific behaviour (:worker service, Compose UI)
# still requires a real device or an Android emulator.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

QEMU="${QEMU_AARCH64:-}"
if [ -z "$QEMU" ]; then
    QEMU="$(command -v qemu-aarch64-static || command -v qemu-aarch64 || true)"
fi
if [ -z "$QEMU" ]; then
    for candidate in \
        "$HOME/tools/qemu-aarch64-static" \
        "$HOME/.local/bin/qemu-aarch64-static" \
        "/usr/bin/qemu-aarch64-static"; do
        if [ -x "$candidate" ]; then
            QEMU="$candidate"
            break
        fi
    done
fi
if [ -z "$QEMU" ] || [ ! -x "$QEMU" ]; then
    echo "error: qemu-aarch64-static was not found." >&2
    echo "Install qemu-user-static, or set QEMU_AARCH64=/path/to/qemu-aarch64-static." >&2
    exit 1
fi

CARGO="${CARGO:-$(command -v cargo || true)}"
if [ -z "$CARGO" ]; then
    CARGO="$HOME/.cargo/bin/cargo"
fi
if [ ! -x "$CARGO" ]; then
    echo "error: cargo was not found." >&2
    exit 1
fi

RUSTUP="${RUSTUP:-$(command -v rustup || true)}"
if [ -z "$RUSTUP" ]; then
    RUSTUP="$HOME/.cargo/bin/rustup"
fi
if [ -x "$RUSTUP" ]; then
    "$RUSTUP" target add aarch64-unknown-linux-musl >/dev/null
fi

cd "$ROOT/core"

export RUSTFLAGS="${RUSTFLAGS:-} -C linker=rust-lld -C link-self-contained=yes"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUNNER="$QEMU"

echo "Running a64core tests for aarch64-unknown-linux-musl under $QEMU"
"$CARGO" test --target aarch64-unknown-linux-musl -- --test-threads=1
