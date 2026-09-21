#!/bin/sh
set -eu
printf 'cwd=%s\n' "$PWD"
printf 'arg1=%s arg2=%s\n' "$1" "$2"
printf 'backend diagnostics\n' >&2
test -f input.i
