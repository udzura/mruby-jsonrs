MRuby::CrossBuild.new("benchmark-picoruby-json-wasi") do |conf|
  conf.toolchain :wasi, target: "wasm32-wasip1"
  conf.ports :posix

  conf.cc.defines << "PICORB_PLATFORM_WASM"
  # mruby-task has an Emscripten-specific path that avoids POSIX signals.
  # WASI likewise has no SIGALRM/setitimer implementation.
  conf.cc.defines << "__EMSCRIPTEN__"
  conf.cc.defines << "MRB_TICK_UNIT=4"
  conf.cc.defines << "MRB_TIMESLICE_TICK_COUNT=1"
  conf.cc.defines << "MRB_32BIT"
  conf.cc.defines << "MRB_INT64"
  conf.cc.defines << "MRB_NO_BOXING"
  conf.cc.defines << "MRB_UTF8_STRING"
  conf.cc.defines << "PICORB_VM_MRUBY"
  conf.cc.defines << "MRB_USE_TASK_SCHEDULER"

  # Equivalent to the non-mutating portion of Build#picoruby. Calling that
  # helper would rewrite PicoRuby's checked-out src/version.c.
  conf.common
  conf.cc.include_paths << "#{MRUBY_ROOT}/mrbgems/picoruby-mruby/include"
  conf.cc.include_paths << "#{MRUBY_ROOT}/mrbgems/picoruby-mruby/lib/mruby/mrbgems/mruby-task/include"

  # PicoRuby's POSIX IO port currently uses fork/wait headers unavailable in
  # WASI. The benchmark runner only needs mruby's file-loading executable.
  # Keep the runtime small and select it directly instead of using the POSIX
  # gemboxes.
  mruby_gems = "#{MRUBY_ROOT}/mrbgems/picoruby-mruby/lib/mruby/mrbgems"
  conf.gem core: "picoruby-mruby"
  conf.gem gemdir: "#{mruby_gems}/mruby-compiler"
  conf.gem gemdir: "#{mruby_gems}/mruby-bin-mrbc"
  conf.gem gemdir: "#{mruby_gems}/mruby-bin-mruby"
  conf.gem gemdir: "#{mruby_gems}/mruby-string-ext"
  conf.gem gemdir: "#{mruby_gems}/mruby-regexp"
  conf.gem File.expand_path("../timer", __dir__)
  conf.gem core: "picoruby-json"
end
