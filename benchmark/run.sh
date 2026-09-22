#!/bin/sh
set -eu

if [ -z "${MRUBY_ROOT:-}" ]; then
  echo "MRUBY_ROOT must point to an mruby checkout" >&2
  exit 1
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
build_dir=${BENCH_BUILD_DIR:-${TMPDIR:-/tmp}/mruby-jsonrs-benchmark}
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
    MRUBY_CONFIG="$config" MRUBY_BUILD_DIR="$build_dir" rake -j"$jobs"
  )
}

run() {
  name=$1
  binary=$2
  echo
  echo "==> $name"
  "$binary" "$project_root/benchmark/json.rb" "$iterations" "$warmup" "$fixture"
}

build "mruby-jsonrs" "$project_root/benchmark/build_config/jsonrs.rb"
build "mattn/mruby-json" "$project_root/benchmark/build_config/mruby_json.rb"

run "mruby-jsonrs" "$build_dir/benchmark-jsonrs/bin/mruby"
run "mattn/mruby-json" "$build_dir/benchmark-mruby-json/bin/mruby"
