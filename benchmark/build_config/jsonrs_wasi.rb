jsonrs_dir = ENV.fetch("MRUBY_JSONRS_GEM_DIR") do
  File.expand_path("../..", __dir__)
end

MRuby::CrossBuild.new("benchmark-jsonrs-wasi") do |conf|
  conf.toolchain :wasi, target: "wasm32-wasip1"
  conf.gem core: "mruby-bin-mrbc"
  conf.gem core: "mruby-bin-mruby"
  conf.gem File.expand_path("../timer", __dir__)
  conf.gem jsonrs_dir
end
