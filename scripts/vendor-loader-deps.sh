#!/usr/bin/env bash
set -euo pipefail

if [[ $# -gt 0 ]]; then
  echo "Usage: ${0##*/}" >&2
  exit 1
fi

VENDOR_DIR="loader/uefi/vendor"
mkdir -p "${VENDOR_DIR}"

cat <<MSG
This script will run 'cargo vendor' for the loader crate once network access is
available.  The generated sources enable offline builds of the UEFI loader.

  cargo vendor --manifest-path loader/uefi/Cargo.toml ${VENDOR_DIR}

After vendoring, you can build the loader offline with:

  cargo build -p loader-uefi --features firmware --offline 

Note: network access is required the first time to populate the vendor tree.
MSG
