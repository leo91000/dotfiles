#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
export WEWEB_PSQL_SELF="${BASH_SOURCE[0]}"
exec cargo +nightly -Zscript "${SCRIPT_DIR}/exec_psql_env.rs" "$@"
