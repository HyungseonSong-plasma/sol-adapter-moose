#!/usr/bin/env bash
# Experimental exec adapter: run pinned MOOSE CLI for Rust backend-only tests.
# Not an accepted SOL adapter backend or a Conda package-identity substitute.
set -euo pipefail
: "${MOOSE_DOCKER_PIN:?missing trusted pinned image}"
: "${GITHUB_WORKSPACE:?missing workspace}"
if [[ ! "$MOOSE_DOCKER_PIN" =~ ^idaholab/moose@sha256:[0-9a-f]{64}$ ]]; then
  echo "MOOSE_DOCKER_PIN must be exact immutable official digest" >&2
  exit 64
fi
workspace="$(realpath "$GITHUB_WORKSPACE")"
working_dir="$(pwd -P)"
case "$working_dir" in
  "$workspace"|"$workspace"/*) ;;
  *)
    echo "Refusing to mount an execution workspace outside repository" >&2
    exit 64
    ;;
esac
exec docker run --rm --pull never --network none \
  --volume "$workspace:$workspace:rw" \
  --workdir "$working_dir" --entrypoint /bin/bash \
  "$MOOSE_DOCKER_PIN" -lc \
  'set -euo pipefail; if [[ -f /environment ]]; then source /environment; fi; exec moose-opt "$@"' \
  bash "$@"
