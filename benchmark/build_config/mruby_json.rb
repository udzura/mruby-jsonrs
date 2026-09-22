MRuby::Build.new("benchmark-mruby-json") do |conf|
  conf.toolchain
  conf.gem core: "mruby-bin-mrbc"
  conf.gem core: "mruby-bin-mruby"
  conf.gem core: "mruby-benchmark"
  conf.gem github: "mattn/mruby-json",
           branch: "master",
           checksum_hash: "f99d9428025469f2400f93c53b185f65f963e507"
end
