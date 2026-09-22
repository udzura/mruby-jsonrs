jsonrs_dir = ENV.fetch("MRUBY_JSONRS_GEM_DIR") do
  File.expand_path("../..", __dir__)
end

MRuby::Build.new("benchmark-jsonrs") do |conf|
  conf.toolchain
  conf.gem core: "mruby-bin-mrbc"
  conf.gem core: "mruby-bin-mruby"
  conf.gem core: "mruby-benchmark"
  conf.gem jsonrs_dir
end
