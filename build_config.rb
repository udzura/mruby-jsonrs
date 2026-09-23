MRuby::Build.new("mruby-jsonrs") do |conf|
  #conf.toolchain
  conf.toolchain :clang
  conf.cc.command = conf.linker.command = "emcc"
  conf.archiver.command = "emar"

  conf.gem core: "mruby-bin-mrbc"
  conf.gem File.expand_path(__dir__)
  conf.enable_test
end
