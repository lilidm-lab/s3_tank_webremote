#!/bin/sh
# Release build with build-time config baked in from .env
# (WIFI_SSID / WIFI_PASS / WS_URL are captured via option_env! at compile time).
# Extra args pass through to cargo, e.g.: ./build.sh --verbose
set -eu

cd "$(dirname "$0")"

ENV_FILE="${ENV_FILE:-.env}"

if [ ! -f "$ENV_FILE" ]; then
  echo "error: $ENV_FILE not found (cp .env.sample $ENV_FILE and fill it in)" >&2
  exit 1
fi

# export KEY=value lines without requiring `export` prefixes in the file
set -a
. "./$ENV_FILE"
set +a

for VAR in WIFI_SSID WIFI_PASS WS_URL; do
  eval "VAL=\${$VAR:-}"
  if [ -z "$VAL" ]; then
    echo "error: $VAR not set in $ENV_FILE" >&2
    exit 1
  fi
done

echo "release build, config from $ENV_FILE (WS_URL=$WS_URL)"
cargo build --release "$@"
