#!/usr/bin/env bash
# Experimental exec adapter: run pinned MOOSE CLI for Rust backend-only tests.
# Not an accepted SOL adapter backend or a Conda package-identity substitute.
set -euo pipefail
: "${MOOSE_DOCKER_PIN:?missing trusted pinned image}"
# Consumer-provided mount root; GitHub checkout root remains compatible.
# For local clients mount only the current working directory by default.
mount_root="${SOL_MOOSE_OCI_MOUNT_ROOT:-${GITHUB_WORKSPACE:-$(pwd -P)}}"
if [[ "$MOOSE_DOCKER_PIN" != 'idaholab/moose@sha256:47fc6c5bff5e86869e7c467f999e9585ece13af9ffd1f580fef5369728a93452' ]]; then
  echo "MOOSE_DOCKER_PIN must equal the separately approved candidate digest" >&2
  exit 64
fi
workspace="$(realpath "$mount_root")"
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
