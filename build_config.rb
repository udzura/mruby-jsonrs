MRuby::Build.new("mruby-jsonrs") do |conf|
  conf.toolchain
  conf.gem core: "mruby-bin-mrbc"
  conf.gem File.expand_path(__dir__)
  conf.enable_test
end
