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

assert("JSON parse error") do
  assert_raise(JSON::ParserError) { JSON.parse("{") }
end

assert("JSON generation error") do
  assert_raise(JSON::GeneratorError) { JSON.generate(Object.new) }
  assert_raise(JSON::GeneratorError) { JSON.generate({1 => "one"}) }
end
