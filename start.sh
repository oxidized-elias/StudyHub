#!/usr/bin/env bash
set -euo pipefail
suite_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec node "$suite_root/server.mjs" "$@"
