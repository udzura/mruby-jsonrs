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
  const char *message;
  char *rust_error;
  mrb_bool raised;
  mrb_value exception;
  mrb_value state;
  struct RClass *error_class;
} jsonrs_generate_context;

typedef struct {
  mruby_jsonrs_value *object;
  jsonrs_generate_context *generate;
  unsigned int depth;
} jsonrs_hash_context;

typedef struct {
  mrb_value receiver;
  mrb_sym method;
  mrb_int argc;
  const mrb_value *argv;
} jsonrs_method_call;

static struct RClass *jsonrs_parser_error;
static struct RClass *jsonrs_generator_error;
static struct RClass *jsonrs_state_class;

static mrb_value
jsonrs_call_method(mrb_state *mrb, void *data)
{
  jsonrs_method_call *call = (jsonrs_method_call *)data;
  return mrb_funcall_argv(mrb, call->receiver, call->method, call->argc, call->argv);
}

static mrb_value
jsonrs_protected_call(mrb_state *mrb, mrb_value receiver, mrb_sym method,
                      mrb_int argc, const mrb_value *argv,
                      jsonrs_generate_context *context)
{
  jsonrs_method_call call = { receiver, method, argc, argv };
  mrb_bool raised = FALSE;
  mrb_value result = mrb_protect_error(mrb, jsonrs_call_method, &call, &raised);
  if (raised) {
    context->raised = TRUE;
    context->exception = result;
  }
  return result;
}

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
                                           jsonrs_generate_context *);

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
    context->generate->message = "JSON object keys must be strings or symbols";
    return 1;
  }

  mruby_jsonrs_value *child = jsonrs_from_mrb(
    mrb, value, context->depth + 1, context->generate);
  if (!child) return 1;

  if (!mruby_jsonrs_object_insert(context->object, (const uint8_t *)key_data,
                                  (size_t)key_len, child,
                                  &context->generate->rust_error)) {
    mruby_jsonrs_value_free(child);
    if (!context->generate->rust_error) {
      context->generate->message = "failed to build JSON object";
    }
    return 1;
  }
  return 0;
}

static mruby_jsonrs_value *
jsonrs_from_mrb(mrb_state *mrb, mrb_value value, unsigned int depth,
                jsonrs_generate_context *context)
{
  if (depth > JSONRS_MAX_NESTING) {
    context->message = "JSON structure is too deeply nested";
    return NULL;
  }
  if (mrb_nil_p(value)) return mruby_jsonrs_null_new();
  if (mrb_true_p(value)) return mruby_jsonrs_bool_new(true);
  if (mrb_false_p(value)) return mruby_jsonrs_bool_new(false);
  if (mrb_integer_p(value)) return mruby_jsonrs_i64_new((int64_t)mrb_integer(value));
#ifndef MRB_NO_FLOAT
  if (mrb_float_p(value)) {
    return mruby_jsonrs_f64_new((double)mrb_float(value), &context->rust_error);
  }
#endif
  if (mrb_string_p(value)) {
    return mruby_jsonrs_string_new((const uint8_t *)RSTRING_PTR(value),
                                   (size_t)RSTRING_LEN(value),
                                   &context->rust_error);
  }
  if (mrb_symbol_p(value)) {
    mrb_int len;
    const char *name = mrb_sym_name_len(mrb, mrb_symbol(value), &len);
    return mruby_jsonrs_string_new((const uint8_t *)name, (size_t)len,
                                   &context->rust_error);
  }
  if (mrb_array_p(value)) {
    mruby_jsonrs_value *array = mruby_jsonrs_array_new();
    mrb_int len = RARRAY_LEN(value);
    for (mrb_int i = 0; i < len; i++) {
      mruby_jsonrs_value *child = jsonrs_from_mrb(
        mrb, RARRAY_PTR(value)[i], depth + 1, context);
      if (!child) {
        mruby_jsonrs_value_free(array);
        return NULL;
      }
      if (!mruby_jsonrs_array_push(array, child)) {
        mruby_jsonrs_value_free(child);
        mruby_jsonrs_value_free(array);
        context->message = "failed to build JSON array";
        return NULL;
      }
    }
    return array;
  }
  if (mrb_hash_p(value)) {
    mruby_jsonrs_value *object = mruby_jsonrs_object_new();
    jsonrs_hash_context hash_context = { object, context, depth };
    mrb_hash_foreach(mrb, mrb_hash_ptr(value), jsonrs_hash_entry, &hash_context);
    if (context->message || context->rust_error || context->raised) {
      mruby_jsonrs_value_free(object);
      return NULL;
    }
    return object;
  }

  int arena = mrb_gc_arena_save(mrb);
  mrb_sym to_json = mrb_intern_lit(mrb, "to_json");
  if (mrb_respond_to(mrb, value, to_json)) {
    mrb_value json = jsonrs_protected_call(
      mrb, value, to_json, 1, &context->state, context);
    if (context->raised) return NULL;
    if (!mrb_string_p(json)) {
      context->message = "to_json must return a String";
      context->error_class = E_TYPE_ERROR;
      mrb_gc_arena_restore(mrb, arena);
      return NULL;
    }
    mruby_jsonrs_value *raw = mruby_jsonrs_raw_new(
      (const uint8_t *)RSTRING_PTR(json), (size_t)RSTRING_LEN(json),
      &context->rust_error);
    mrb_gc_arena_restore(mrb, arena);
    return raw;
  }

  mrb_value string = jsonrs_protected_call(
    mrb, value, mrb_intern_lit(mrb, "to_s"), 0, NULL, context);
  if (context->raised) return NULL;
  if (!mrb_string_p(string)) {
    context->message = "to_s must return a String";
    context->error_class = E_TYPE_ERROR;
    mrb_gc_arena_restore(mrb, arena);
    return NULL;
  }
  mruby_jsonrs_value *fallback = mruby_jsonrs_string_new(
    (const uint8_t *)RSTRING_PTR(string), (size_t)RSTRING_LEN(string),
    &context->rust_error);
  mrb_gc_arena_restore(mrb, arena);
  return fallback;
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
  mrb_value input;
  mrb_get_args(mrb, "o", &input);

  mrb_value state = mrb_obj_is_kind_of(mrb, self, jsonrs_state_class)
    ? self
    : mrb_obj_new(mrb, jsonrs_state_class, 0, NULL);
  jsonrs_generate_context context = {
    NULL, NULL, FALSE, mrb_nil_value(), state, jsonrs_generator_error
  };
  mruby_jsonrs_value *value = jsonrs_from_mrb(mrb, input, 0, &context);
  if (!value) {
    if (context.raised) mrb_exc_raise(mrb, context.exception);
    if (context.rust_error) {
      jsonrs_raise_rust_error(mrb, jsonrs_generator_error, context.rust_error);
    }
    mrb_exc_raise(mrb, jsonrs_exception(
      mrb, context.error_class, context.message));
  }

  uint8_t *output = NULL;
  size_t output_len = 0;
  if (!mruby_jsonrs_generate(
        value, &output, &output_len, &context.rust_error)) {
    mruby_jsonrs_value_free(value);
    jsonrs_raise_rust_error(
      mrb, jsonrs_generator_error, context.rust_error);
  }
  mruby_jsonrs_value_free(value);

  mrb_value result = mrb_str_new(mrb, (const char *)output, (mrb_int)output_len);
  mruby_jsonrs_bytes_free(output, output_len);
  return result;
}

static mrb_value
jsonrs_state_from_state(mrb_state *mrb, mrb_value self)
{
  (void)self;
  mrb_value state = mrb_nil_value();
  mrb_int argc = mrb_get_args(mrb, "|o", &state);
  if (argc > 0 && mrb_obj_is_kind_of(mrb, state, jsonrs_state_class)) {
    return state;
  }
  return mrb_obj_new(mrb, jsonrs_state_class, 0, NULL);
}

void
mrb_mruby_jsonrs_gem_init(mrb_state *mrb)
{
  struct RClass *json = mrb_define_module(mrb, "JSON");
  struct RClass *json_error = mrb_define_class_under(mrb, json, "JSONError", E_STANDARD_ERROR);
  jsonrs_parser_error = mrb_define_class_under(mrb, json, "ParserError", json_error);
  jsonrs_generator_error = mrb_define_class_under(mrb, json, "GeneratorError", json_error);
  jsonrs_state_class = mrb_define_class_under(mrb, json, "State", mrb->object_class);

  mrb_define_class_method(mrb, jsonrs_state_class, "from_state",
                          jsonrs_state_from_state, MRB_ARGS_OPT(1));
  mrb_define_method(mrb, jsonrs_state_class, "generate",
                    jsonrs_generate, MRB_ARGS_REQ(1));

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
