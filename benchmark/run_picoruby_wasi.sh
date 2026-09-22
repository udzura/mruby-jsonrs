#!/bin/sh
set -eu

if [ -z "${PICORUBY_ROOT:-}" ]; then
  echo "PICORUBY_ROOT must point to a PicoRuby checkout" >&2
  exit 1
fi
if [ -z "${WASI_SDK_PATH:-}" ]; then
  echo "WASI_SDK_PATH must point to wasi-sdk 26 or later" >&2
  exit 1
fi
if ! command -v wasmtime >/dev/null 2>&1; then
  echo "wasmtime is required" >&2
  exit 1
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
build_dir=${BENCH_BUILD_DIR:-${TMPDIR:-/tmp}/mruby-jsonrs-picoruby-wasi-benchmark}
iterations=${ITERATIONS:-1000}
warmup=${WARMUP:-100}
fixture=${FIXTURE:-ascii}
jobs=${JOBS:-4}

echo "==> Building picoruby-json (WASI)"
(
  cd "$PICORUBY_ROOT"
  WASI_SDK_PATH="$WASI_SDK_PATH" \
    MRUBY_CONFIG="$project_root/benchmark/build_config/picoruby_json_wasi.rb" \
    MRUBY_BUILD_DIR="$build_dir" \
    rake -j"$jobs"
)

echo
echo "==> picoruby-json (Wasmtime)"
(
  cd "$project_root"
  wasmtime run --dir . \
    "$build_dir/benchmark-picoruby-json-wasi/bin/mruby" \
    benchmark/json.rb "$iterations" "$warmup" "$fixture"
)
