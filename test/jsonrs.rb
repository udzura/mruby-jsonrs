assert("JSON.parse primitives") do
  assert_equal nil, JSON.parse("null")
  assert_equal true, JSON.parse("true")
  assert_equal false, JSON.parse("false")
  assert_equal 42, JSON.parse("42")
  assert_equal 3.5, JSON.parse("3.5")
  assert_equal "hello", JSON.parse('"hello"')
end

assert("JSON.parse nested values") do
  value = JSON.parse('{"users":[{"name":"Alice","active":true}]}')
  assert_equal "Alice", value["users"][0]["name"]
  assert_equal true, value["users"][0]["active"]
end

assert("JSON.generate primitives and collections") do
  assert_equal "null", JSON.generate(nil)
  assert_equal "true", JSON.generate(true)
  assert_equal '"hello"', JSON.generate("hello")
  assert_equal "[1,2,3]", JSON.generate([1, 2, 3])
  assert_equal '{"answer":42}', JSON.generate({answer: 42})
end

assert("JSON aliases") do
  assert_equal [1, 2], JSON.load("[1,2]")
  assert_equal '{"ok":true}', JSON.dump({ok: true})
end

class JsonrsCustomObject
  def to_json(*args)
    raise "expected one state argument" unless args.length == 1
    raise "expected JSON::State" unless args[0].is_a?(JSON::State)
    '{ "custom" : true }'
  end
end

class JsonrsFallbackObject
  def to_s
    'fallback "value"'
  end
end

class JsonrsInvalidFragment
  def to_json(*_args)
    "not-json"
  end
end

class JsonrsBadJsonReturn
  def to_json(*_args)
    {value: true}
  end
end

class JsonrsFailingJson
  def to_json(*_args)
    raise "custom failure"
  end
end

class JsonrsBadStringReturn
  def to_s
    1
  end
end

assert("JSON.generate uses custom to_json output verbatim") do
  assert_equal '{ "custom" : true }', JSON.generate(JsonrsCustomObject.new)
  assert_equal '[{ "custom" : true }]', JSON.generate([JsonrsCustomObject.new])
  assert_equal '{"item":{ "custom" : true }}', JSON.generate({item: JsonrsCustomObject.new})
  assert_equal "not-json", JSON.generate(JsonrsInvalidFragment.new)
end

assert("JSON::State can generate values from to_json") do
  state = JSON::State.new
  assert_true state.equal?(JSON::State.from_state(state))
  assert_true JSON::State.from_state(nil).is_a?(JSON::State)

  custom = Class.new do
    def to_json(state = nil, *_args)
      JSON::State.from_state(state).generate({wrapped: true})
    end
  end
  assert_equal '{"wrapped":true}', JSON.generate(custom.new)
end

assert("JSON.generate falls back to to_s") do
  assert_equal '"fallback \\"value\\""', JSON.generate(JsonrsFallbackObject.new)
end

assert("JSON.generate validates to_json return type") do
  assert_raise(TypeError) { JSON.generate(JsonrsBadJsonReturn.new) }
  assert_raise(TypeError) { JSON.generate(JsonrsBadStringReturn.new) }
end

assert("JSON.generate propagates to_json exceptions") do
  error = assert_raise(RuntimeError) { JSON.generate(JsonrsFailingJson.new) }
  assert_equal "custom failure", error.message
end

assert("JSON parse error") do
  assert_raise(JSON::ParserError) { JSON.parse("{") }
end

assert("JSON generation error") do
  assert_raise(JSON::GeneratorError) { JSON.generate({1 => "one"}) }
end
