MRuby::CrossBuild.new("benchmark-mruby-json-wasi") do |conf|
  conf.toolchain :wasi, target: "wasm32-wasip1"
  conf.gem core: "mruby-bin-mrbc"
  conf.gem core: "mruby-bin-mruby"
  conf.gem File.expand_path("../timer", __dir__)
  conf.gem github: "mattn/mruby-json",
           branch: "master",
           checksum_hash: "f99d9428025469f2400f93c53b185f65f963e507"
end
