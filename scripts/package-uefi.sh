#!/usr/bin/env bash
set -euo pipefail

PROFILE="debug"
ARTIFACT_DIR="build/efi"
BOOTFS_ROOT="services/init/bootfs"

usage() {
  cat <<USAGE
Usage: ${0##*/} [--release]

Builds the Rustcore kernel and assembles a minimal EFI payload structure:
  ${ARTIFACT_DIR}/EFI/RUSTCORE/kernel.elf
  ${ARTIFACT_DIR}/EFI/RUSTCORE/bootfs.bin
USAGE
}

if [[ ${1:-} == "--release" ]]; then
  PROFILE="release"
  shift
fi

if [[ $# -gt 0 ]]; then
  usage
  exit 1
fi

BUILD_TARGET="targets/x86_64-rustcore.json"

cargo +nightly build --target "${BUILD_TARGET}"
cargo +nightly build -p init --target "${BUILD_TARGET}"

KERNEL_BIN="target/x86_64-rustcore/${PROFILE}/kernel"
if [[ ! -f ${KERNEL_BIN} ]]; then
  echo "kernel binary not found at ${KERNEL_BIN}" >&2
  exit 1
fi

DEST_DIR="${ARTIFACT_DIR}/EFI/RUSTCORE"
mkdir -p "${DEST_DIR}"
cp "${KERNEL_BIN}" "${DEST_DIR}/kernel.elf"

python3 scripts/build-bootfs.py "${BOOTFS_ROOT}" "${DEST_DIR}/bootfs.bin"

echo "EFI payload staged at ${DEST_DIR}" >&2
