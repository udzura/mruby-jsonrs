#include <mruby.h>
#include <mruby/array.h>
#include <mruby/class.h>
#include <mruby/error.h>
#include <mruby/hash.h>
#include <mruby/string.h>

#include <stdint.h>

#include "jsonrs.h"

#define JSONRS_MAX_NESTING 128

typedef struct {
  mruby_jsonrs_value *object;
  const char *message;
  char *rust_error;
  unsigned int depth;
} jsonrs_hash_context;

static struct RClass *jsonrs_parser_error;
static struct RClass *jsonrs_generator_error;

static mrb_value
jsonrs_exception(mrb_state *mrb, struct RClass *klass, const char *message)
{
  mrb_value text = mrb_str_new_cstr(mrb, message ? message : "JSON operation failed");
  return mrb_exc_new_str(mrb, klass, text);
}

static void
jsonrs_raise_rust_error(mrb_state *mrb, struct RClass *klass, char *error)
{
  mrb_value exception = jsonrs_exception(mrb, klass, error);
  mruby_jsonrs_error_free(error);
  mrb_exc_raise(mrb, exception);
}

static mruby_jsonrs_value *jsonrs_from_mrb(mrb_state *, mrb_value, unsigned int,
                                           const char **, char **);

static int
jsonrs_hash_entry(mrb_state *mrb, mrb_value key, mrb_value value, void *data)
{
  jsonrs_hash_context *context = (jsonrs_hash_context *)data;
  const char *key_data;
  mrb_int key_len;

  if (mrb_string_p(key)) {
    key_data = RSTRING_PTR(key);
    key_len = RSTRING_LEN(key);
  }
  else if (mrb_symbol_p(key)) {
    key_data = mrb_sym_name_len(mrb, mrb_symbol(key), &key_len);
  }
  else {
    context->message = "JSON object keys must be strings or symbols";
    return 1;
  }

  mruby_jsonrs_value *child = jsonrs_from_mrb(mrb, value, context->depth + 1,
                                               &context->message,
                                               &context->rust_error);
  if (!child) return 1;

  if (!mruby_jsonrs_object_insert(context->object, (const uint8_t *)key_data,
                                  (size_t)key_len, child,
                                  &context->rust_error)) {
    mruby_jsonrs_value_free(child);
    if (!context->rust_error) context->message = "failed to build JSON object";
    return 1;
  }
  return 0;
}

static mruby_jsonrs_value *
jsonrs_from_mrb(mrb_state *mrb, mrb_value value, unsigned int depth,
                const char **message, char **rust_error)
{
  if (depth > JSONRS_MAX_NESTING) {
    *message = "JSON structure is too deeply nested";
    return NULL;
  }
  if (mrb_nil_p(value)) return mruby_jsonrs_null_new();
  if (mrb_true_p(value)) return mruby_jsonrs_bool_new(true);
  if (mrb_false_p(value)) return mruby_jsonrs_bool_new(false);
  if (mrb_integer_p(value)) return mruby_jsonrs_i64_new((int64_t)mrb_integer(value));
#ifndef MRB_NO_FLOAT
  if (mrb_float_p(value)) return mruby_jsonrs_f64_new((double)mrb_float(value), rust_error);
#endif
  if (mrb_string_p(value)) {
    return mruby_jsonrs_string_new((const uint8_t *)RSTRING_PTR(value),
                                   (size_t)RSTRING_LEN(value), rust_error);
  }
  if (mrb_symbol_p(value)) {
    mrb_int len;
    const char *name = mrb_sym_name_len(mrb, mrb_symbol(value), &len);
    return mruby_jsonrs_string_new((const uint8_t *)name, (size_t)len, rust_error);
  }
  if (mrb_array_p(value)) {
    mruby_jsonrs_value *array = mruby_jsonrs_array_new();
    mrb_int len = RARRAY_LEN(value);
    for (mrb_int i = 0; i < len; i++) {
      mruby_jsonrs_value *child = jsonrs_from_mrb(
        mrb, RARRAY_PTR(value)[i], depth + 1, message, rust_error);
      if (!child) {
        mruby_jsonrs_value_free(array);
        return NULL;
      }
      if (!mruby_jsonrs_array_push(array, child)) {
        mruby_jsonrs_value_free(child);
        mruby_jsonrs_value_free(array);
        *message = "failed to build JSON array";
        return NULL;
      }
    }
    return array;
  }
  if (mrb_hash_p(value)) {
    mruby_jsonrs_value *object = mruby_jsonrs_object_new();
    jsonrs_hash_context context = { object, NULL, NULL, depth };
    mrb_hash_foreach(mrb, mrb_hash_ptr(value), jsonrs_hash_entry, &context);
    if (context.message || context.rust_error) {
      mruby_jsonrs_value_free(object);
      *message = context.message;
      *rust_error = context.rust_error;
      return NULL;
    }
    return object;
  }

  *message = "object is not JSON serializable";
  return NULL;
}

static mrb_value
jsonrs_to_mrb(mrb_state *mrb, const mruby_jsonrs_value *value, unsigned int depth,
              const char **message)
{
  if (depth > JSONRS_MAX_NESTING) {
    *message = "JSON structure is too deeply nested";
    return mrb_nil_value();
  }

  switch (mruby_jsonrs_value_type(value)) {
    case MRUBY_JSONRS_NULL:
      return mrb_nil_value();
    case MRUBY_JSONRS_BOOL:
      return mrb_bool_value(mruby_jsonrs_bool_get(value));
    case MRUBY_JSONRS_I64: {
      int64_t number = mruby_jsonrs_i64_get(value);
#if MRB_INT_MAX < INT64_MAX
      if (number > MRB_INT_MAX || number < MRB_INT_MIN) {
        *message = "JSON integer is outside the mruby integer range";
        return mrb_nil_value();
      }
#endif
      return mrb_int_value(mrb, (mrb_int)number);
    }
    case MRUBY_JSONRS_U64: {
      uint64_t number = mruby_jsonrs_u64_get(value);
      if (number > (uint64_t)MRB_INT_MAX) {
        *message = "JSON integer is outside the mruby integer range";
        return mrb_nil_value();
      }
      return mrb_int_value(mrb, (mrb_int)number);
    }
    case MRUBY_JSONRS_F64:
#ifdef MRB_NO_FLOAT
      *message = "JSON floating-point numbers are not supported by this mruby build";
      return mrb_nil_value();
#else
      return mrb_float_value(mrb, (mrb_float)mruby_jsonrs_f64_get(value));
#endif
    case MRUBY_JSONRS_STRING: {
      size_t len = 0;
      const uint8_t *data = mruby_jsonrs_string_data(value, &len);
      return mrb_str_new(mrb, (const char *)data, (mrb_int)len);
    }
    case MRUBY_JSONRS_ARRAY: {
      size_t len = mruby_jsonrs_array_len(value);
      mrb_value array = mrb_ary_new_capa(mrb, (mrb_int)len);
      for (size_t i = 0; i < len; i++) {
        mrb_value child = jsonrs_to_mrb(
          mrb, mruby_jsonrs_array_get(value, i), depth + 1, message);
        if (*message) return mrb_nil_value();
        mrb_ary_push(mrb, array, child);
      }
      return array;
    }
    case MRUBY_JSONRS_OBJECT: {
      size_t len = mruby_jsonrs_object_len(value);
      mrb_value object = mrb_hash_new_capa(mrb, (mrb_int)len);
      for (size_t i = 0; i < len; i++) {
        size_t key_len = 0;
        const uint8_t *key_data = mruby_jsonrs_object_key(value, i, &key_len);
        mrb_value key = mrb_str_new(mrb, (const char *)key_data, (mrb_int)key_len);
        mrb_value child = jsonrs_to_mrb(
          mrb, mruby_jsonrs_object_value(value, i), depth + 1, message);
        if (*message) return mrb_nil_value();
        mrb_hash_set(mrb, object, key, child);
      }
      return object;
    }
    default:
      *message = "invalid value returned by JSON parser";
      return mrb_nil_value();
  }
}

static mrb_value
jsonrs_parse(mrb_state *mrb, mrb_value self)
{
  (void)self;
  char *input;
  mrb_int input_len;
  char *error = NULL;
  mrb_get_args(mrb, "s", &input, &input_len);

  mruby_jsonrs_value *value = mruby_jsonrs_parse(
    (const uint8_t *)input, (size_t)input_len, &error);
  if (!value) jsonrs_raise_rust_error(mrb, jsonrs_parser_error, error);

  const char *message = NULL;
  mrb_value result = jsonrs_to_mrb(mrb, value, 0, &message);
  mruby_jsonrs_value_free(value);
  if (message) mrb_exc_raise(mrb, jsonrs_exception(mrb, jsonrs_parser_error, message));
  return result;
}

static mrb_value
jsonrs_generate(mrb_state *mrb, mrb_value self)
{
  (void)self;
  mrb_value input;
  const char *message = NULL;
  char *error = NULL;
  mrb_get_args(mrb, "o", &input);

  mruby_jsonrs_value *value = jsonrs_from_mrb(mrb, input, 0, &message, &error);
  if (!value) {
    if (error) jsonrs_raise_rust_error(mrb, jsonrs_generator_error, error);
    mrb_exc_raise(mrb, jsonrs_exception(mrb, jsonrs_generator_error, message));
  }

  uint8_t *output = NULL;
  size_t output_len = 0;
  if (!mruby_jsonrs_generate(value, &output, &output_len, &error)) {
    mruby_jsonrs_value_free(value);
    jsonrs_raise_rust_error(mrb, jsonrs_generator_error, error);
  }
  mruby_jsonrs_value_free(value);

  mrb_value result = mrb_str_new(mrb, (const char *)output, (mrb_int)output_len);
  mruby_jsonrs_bytes_free(output, output_len);
  return result;
}

void
mrb_mruby_jsonrs_gem_init(mrb_state *mrb)
{
  struct RClass *json = mrb_define_module(mrb, "JSON");
  struct RClass *json_error = mrb_define_class_under(mrb, json, "JSONError", E_STANDARD_ERROR);
  jsonrs_parser_error = mrb_define_class_under(mrb, json, "ParserError", json_error);
  jsonrs_generator_error = mrb_define_class_under(mrb, json, "GeneratorError", json_error);

  mrb_define_module_function(mrb, json, "parse", jsonrs_parse, MRB_ARGS_REQ(1));
  mrb_define_module_function(mrb, json, "load", jsonrs_parse, MRB_ARGS_REQ(1));
  mrb_define_module_function(mrb, json, "generate", jsonrs_generate, MRB_ARGS_REQ(1));
  mrb_define_module_function(mrb, json, "dump", jsonrs_generate, MRB_ARGS_REQ(1));
}

void
mrb_mruby_jsonrs_gem_final(mrb_state *mrb)
{
  (void)mrb;
}
