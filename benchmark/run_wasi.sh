#!/bin/sh
set -eu

if [ -z "${MRUBY_ROOT:-}" ]; then
  echo "MRUBY_ROOT must point to an mruby checkout" >&2
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
build_dir=${BENCH_BUILD_DIR:-${TMPDIR:-/tmp}/mruby-jsonrs-wasi-benchmark}
iterations=${ITERATIONS:-1000}
warmup=${WARMUP:-100}
fixture=${FIXTURE:-ascii}
jobs=${JOBS:-4}

build() {
  name=$1
  config=$2
  echo "==> Building $name"
  (
    cd "$MRUBY_ROOT"
    WASI_SDK_PATH="$WASI_SDK_PATH" \
      MRUBY_JSONRS_RUST_TARGET=wasm32-wasip1 \
      MRUBY_CONFIG="$config" \
      MRUBY_BUILD_DIR="$build_dir" \
      rake -j"$jobs"
  )
}

run() {
  name=$1
  binary=$2
  echo
  echo "==> $name"
  (
    cd "$project_root"
    wasmtime run --dir . "$binary" benchmark/json.rb "$iterations" "$warmup" "$fixture"
  )
}

build "mruby-jsonrs (WASI)" "$project_root/benchmark/build_config/jsonrs_wasi.rb"
build "mattn/mruby-json (WASI)" "$project_root/benchmark/build_config/mruby_json_wasi.rb"

run "mruby-jsonrs (Wasmtime)" "$build_dir/benchmark-jsonrs-wasi/bin/mruby"
run "mattn/mruby-json (Wasmtime)" "$build_dir/benchmark-mruby-json-wasi/bin/mruby"
