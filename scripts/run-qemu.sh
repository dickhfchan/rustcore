#!/usr/bin/env bash
set -euo pipefail

QEMU_BIN=${QEMU_BIN:-"qemu-system-x86_64"}
MACHINE=${MACHINE:-"q35"}
CPU=${CPU:-"qemu64"}
DEBUG_LOG=${DEBUG_LOG:-"debug.log"}
MODE="kernel"
TARGET_BIN=""

if [[ $# -gt 0 && -f "$1" ]]; then
    MODE="test"
    TARGET_BIN="$1"
    shift
fi

if [[ "$MODE" == "kernel" ]]; then
    BUILD_PROFILE="debug"
    if [[ ${1:-} == "--release" ]]; then
        BUILD_PROFILE="release"
        shift
    fi
    TARGET_BIN=${KERNEL_ELF:-"target/x86_64-rustcore/${BUILD_PROFILE}/kernel"}
fi

if ! command -v "$QEMU_BIN" >/dev/null 2>&1; then
    echo "QEMU binary '$QEMU_BIN' not available; skipping ${MODE} run" >&2
    exit 0
fi

if [[ "$MODE" == "test" ]]; then
    echo "QEMU test harness placeholder for $TARGET_BIN" >&2
    exit 0
fi

>"${DEBUG_LOG}"

echo "Launching QEMU with kernel image: ${TARGET_BIN}" >&2
echo "Debug log will be written to: ${DEBUG_LOG}" >&2
exec "$QEMU_BIN" \
  -machine "$MACHINE" \
  -cpu "$CPU" \
  -display none \
  -no-reboot \
  -serial stdio \
  -debugcon file:"${DEBUG_LOG}" \
  -global isa-debugcon.iobase=0xe9 \
  -kernel "$TARGET_BIN"
