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
