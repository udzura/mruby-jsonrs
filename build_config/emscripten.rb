MRuby::CrossBuild.new("mruby-jsonrs-emscripten") do |conf|
  conf.toolchain :emscripten
  conf.gem core: "mruby-bin-mruby"
  conf.gem File.expand_path("..", __dir__)
end
