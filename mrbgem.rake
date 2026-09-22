MRuby::Gem::Specification.new("mruby-jsonrs") do |spec|
  spec.license = "MIT"
  spec.authors = "Uchio Kondo"
  spec.version = "0.1.0"
  spec.summary = "JSON support for mruby using Rust and Serde"

  rust_dir = File.join(dir, "rust")
  rust_target = ENV["MRUBY_JSONRS_RUST_TARGET"]
  cc_command = Array(spec.build.cc.command).join(" ")
  rust_target ||= "wasm32-unknown-emscripten" if cc_command.match?(/(?:^|\/)emcc(?:\s|$)/)

  rust_profile_dir = rust_target ? File.join(rust_dir, "target", rust_target, "release") : File.join(rust_dir, "target", "release")
  rust_lib = File.join(rust_profile_dir, "libmruby_jsonrs.a")
  rust_sources = [File.join(rust_dir, "Cargo.toml"), File.join(rust_dir, "Cargo.lock")] + Dir.glob(File.join(rust_dir, "src", "**", "*.rs"))

  file rust_lib => rust_sources do
    command = ["cargo", "build", "--manifest-path", File.join(rust_dir, "Cargo.toml"), "--release"]
    command += ["--target", rust_target] if rust_target
    sh(*command)
  end

  # The shim object depends on the Rust archive so Rust changes also force a
  # final relink. The archives remain separate on the linker command line.
  jsonrs_source = File.join(dir, "src", "jsonrs.c")
  jsonrs_header = File.join(dir, "src", "jsonrs.h")
  jsonrs_object = spec.objs.find { |object| File.basename(object) == "jsonrs#{spec.build.exts.object}" }
  file jsonrs_object => [jsonrs_source, jsonrs_header, __FILE__, rust_lib] do |task|
    spec.cc.run(task.name, jsonrs_source)
  end

  spec.linker.library_paths << rust_profile_dir
  spec.linker.libraries << "mruby_jsonrs"
end
