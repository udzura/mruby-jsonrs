iterations = (ARGV[0] || "1000").to_i
warmup = (ARGV[1] || "100").to_i
fixture = ARGV[2] || "ascii"

# picoruby-json detects its Wasm regexp fast path through this constant.
# The minimal standalone PicoRuby benchmark runner does not load the usual
# picoruby-require gem that defines it.
RUBY_DESCRIPTION = "wasm32-wasip1" unless defined?(RUBY_DESCRIPTION)

# PicoRuby's default Wasm fast path delegates Regexp to a JavaScript host API,
# which does not exist in a Wasmtime/WASI module. Benchmark its portable parser
# path instead.
JSON.use_regexp = false if JSON.respond_to?(:use_regexp=)

raise ArgumentError, "iterations must be positive" unless iterations > 0
raise ArgumentError, "warmup must not be negative" unless warmup >= 0

body = case fixture
when "ascii"
  "ASCII JSON benchmark payload. " * 170
when "ja"
  "日本語を含むJSONベンチマーク本文です。" * 93
else
  raise ArgumentError, "fixture must be ascii or ja"
end

users = []
100.times do |index|
  users << {
    "id" => index,
    "name" => "user-#{index}",
    "active" => index % 3 != 0,
    "scores" => [index, index * 2, index.to_f / 3],
    "metadata" => {
      "role" => index % 2 == 0 ? "admin" : "member",
      "note" => "JSON benchmark note #{index}"
    },
    "nullable" => nil
  }
end

payload = {
  "version" => 1,
  "users" => users,
  "flags" => [true, false, nil],
  "body" => body
}
json = JSON.generate(payload)
parsed = JSON.parse(json)
raise "JSON round trip failed" unless parsed.is_a?(Hash) && parsed["version"] == 1

warmup.times do
  JSON.generate(payload)
  JSON.parse(json)
end

print "fixture: #{fixture}, payload: #{json.bytesize} bytes, iterations: #{iterations}, warmup: #{warmup}\n"
if defined?(Benchmark)
  Benchmark.bm(12) do |benchmark|
    benchmark.report("generate") do
      iterations.times { JSON.generate(payload) }
    end
    benchmark.report("parse") do
      iterations.times { JSON.parse(json) }
    end
  end
else
  started = BenchmarkTimer.monotonic
  iterations.times { JSON.generate(payload) }
  generate_time = BenchmarkTimer.monotonic - started

  started = BenchmarkTimer.monotonic
  iterations.times { JSON.parse(json) }
  parse_time = BenchmarkTimer.monotonic - started

  print "generate: ", generate_time, " seconds\n"
  print "parse:    ", parse_time, " seconds\n"
end
