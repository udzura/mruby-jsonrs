# mruby-jsonrs

`mruby-jsonrs` implements JSON parsing and generation for mruby with Rust,
Serde, and `serde_json`.

The mruby-facing extension is a small C shim. It converts mruby values to and
from an opaque `serde_json::Value`; Rust never depends on the layout of
`mrb_state` or `mrb_value`.

## API

```ruby
JSON.parse('{"answer":42}')
JSON.load('[true, false, null]')
JSON.generate({answer: 42})
JSON.dump([1, 2, 3])
```

The initial implementation supports `nil`, booleans, integers, floats,
strings, symbols, arrays, and hashes. Hash keys must be strings or symbols.
Other objects use their `to_json` result verbatim when the method is defined;
otherwise, their `to_s` result is encoded as a JSON string. `to_json` receives
a `JSON::State` object with `generate` and `JSON::State.from_state` support.

## Add to an mruby build

```ruby
conf.gem github: "udzura/mruby-jsonrs"
```

The gem invokes Cargo automatically. Native builds use the host Rust target.
When mruby's C compiler is `emcc`, Cargo uses
`wasm32-unknown-emscripten`, producing a Rust static library that is added to
the final linker command.

Install the Emscripten Rust target before a Wasm build:

```console
rustup target add wasm32-unknown-emscripten
```

Set `MRUBY_JSONRS_RUST_TARGET` to override the detected Rust target.

An Emscripten development configuration is included as
`build_config/emscripten.rb`:

```console
cd ../mruby
MRUBY_CONFIG=/path/to/mruby-jsonrs/build_config/emscripten.rb rake
```

## Development

Run the Rust tests with:

```console
cargo test --manifest-path rust/Cargo.toml
```

The mruby tests are in `test/jsonrs.rb` and run as part of an mruby test build
that includes this gem.

With an mruby checkout at `../mruby`, the included development configuration
can be used as follows:

```console
cd ../mruby
MRUBY_CONFIG=/path/to/mruby-jsonrs/build_config.rb rake test
```

## Native benchmark

The benchmark builds two native mruby executables with the same configuration,
changing only the JSON gem, and measures generation and parsing of the same
JSON document. The comparison uses `mattn/mruby-json` at commit
`f99d9428025469f2400f93c53b185f65f963e507`.

```console
MRUBY_ROOT=/path/to/mruby ./benchmark/run.sh
```

The default run uses 100 warmup iterations and 1,000 measured iterations.
They and the temporary build directory can be overridden:

```console
MRUBY_ROOT=/path/to/mruby \
  WARMUP=200 ITERATIONS=5000 \
  BENCH_BUILD_DIR=/tmp/mruby-json-benchmark \
  ./benchmark/run.sh
```

### Wasmtime benchmark

The WASI benchmark builds both implementations for `wasm32-wasip1` and runs
them with Wasmtime. A small benchmark-only C shim measures the JSON loops with
`clock_gettime(CLOCK_MONOTONIC)` inside the guest, excluding Wasmtime startup
and compilation from the reported time.

```console
MRUBY_ROOT=/path/to/mruby \
  WASI_SDK_PATH=/path/to/wasi-sdk \
  ./benchmark/run_wasi.sh
```

`WARMUP`, `ITERATIONS`, `JOBS`, and `BENCH_BUILD_DIR` can be overridden in the
same way as for the native benchmark. WASI SDK 26 or later and the Rust
`wasm32-wasip1` target are required.

Set `FIXTURE=ja` to use a roughly 20 KB JSON payload with a Japanese article
body. The default `FIXTURE=ascii` uses an ASCII body of a similar byte length.

### PicoRuby comparison

`picoruby-json` can be measured separately with the same Ruby fixture and the
same in-guest clock. This uses a non-mutating PicoRuby build configuration, so
it does not rewrite files in the supplied checkout.

PicoRuby's production Emscripten build delegates its Wasm `Regexp` fast path
to JavaScript. Wasmtime provides no JavaScript host API, so this benchmark
selects `picoruby-json`'s portable, non-Regexp parser path.

```console
PICORUBY_ROOT=/path/to/picoruby \
  WASI_SDK_PATH=/path/to/wasi-sdk \
  ./benchmark/run_picoruby_wasi.sh
```

Use the same `FIXTURE`, `WARMUP`, `ITERATIONS`, `JOBS`, and `BENCH_BUILD_DIR`
environment variables as the other benchmark commands.
