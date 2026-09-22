#ifndef MRUBY_JSONRS_H
#define MRUBY_JSONRS_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct mruby_jsonrs_value mruby_jsonrs_value;

enum mruby_jsonrs_type {
  MRUBY_JSONRS_NULL = 0,
  MRUBY_JSONRS_BOOL = 1,
  MRUBY_JSONRS_I64 = 2,
  MRUBY_JSONRS_U64 = 3,
  MRUBY_JSONRS_F64 = 4,
  MRUBY_JSONRS_STRING = 5,
  MRUBY_JSONRS_ARRAY = 6,
  MRUBY_JSONRS_OBJECT = 7
};

void mruby_jsonrs_error_free(char *error);
void mruby_jsonrs_value_free(mruby_jsonrs_value *value);
mruby_jsonrs_value *mruby_jsonrs_null_new(void);
mruby_jsonrs_value *mruby_jsonrs_bool_new(bool value);
mruby_jsonrs_value *mruby_jsonrs_i64_new(int64_t value);
mruby_jsonrs_value *mruby_jsonrs_f64_new(double value, char **error);
mruby_jsonrs_value *mruby_jsonrs_string_new(const uint8_t *data, size_t len, char **error);
mruby_jsonrs_value *mruby_jsonrs_array_new(void);
bool mruby_jsonrs_array_push(mruby_jsonrs_value *array, mruby_jsonrs_value *child);
mruby_jsonrs_value *mruby_jsonrs_object_new(void);
bool mruby_jsonrs_object_insert(mruby_jsonrs_value *object, const uint8_t *key,
                                size_t key_len, mruby_jsonrs_value *child,
                                char **error);
mruby_jsonrs_value *mruby_jsonrs_parse(const uint8_t *data, size_t len, char **error);
bool mruby_jsonrs_generate(const mruby_jsonrs_value *value, uint8_t **output,
                           size_t *output_len, char **error);
void mruby_jsonrs_bytes_free(uint8_t *data, size_t len);

int32_t mruby_jsonrs_value_type(const mruby_jsonrs_value *value);
bool mruby_jsonrs_bool_get(const mruby_jsonrs_value *value);
int64_t mruby_jsonrs_i64_get(const mruby_jsonrs_value *value);
uint64_t mruby_jsonrs_u64_get(const mruby_jsonrs_value *value);
double mruby_jsonrs_f64_get(const mruby_jsonrs_value *value);
const uint8_t *mruby_jsonrs_string_data(const mruby_jsonrs_value *value, size_t *len);
size_t mruby_jsonrs_array_len(const mruby_jsonrs_value *value);
const mruby_jsonrs_value *mruby_jsonrs_array_get(const mruby_jsonrs_value *value,
                                                 size_t index);
size_t mruby_jsonrs_object_len(const mruby_jsonrs_value *value);
const uint8_t *mruby_jsonrs_object_key(const mruby_jsonrs_value *value,
                                       size_t index, size_t *len);
const mruby_jsonrs_value *mruby_jsonrs_object_value(const mruby_jsonrs_value *value,
                                                    size_t index);

#endif
