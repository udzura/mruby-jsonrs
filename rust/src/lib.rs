use serde_json::{Number, Value as SerdeValue};
use std::collections::BTreeMap;
use std::ffi::{CString, c_char};
use std::ptr;
use std::slice;

const JSONRS_NULL: i32 = 0;
const JSONRS_BOOL: i32 = 1;
const JSONRS_I64: i32 = 2;
const JSONRS_U64: i32 = 3;
const JSONRS_F64: i32 = 4;
const JSONRS_STRING: i32 = 5;
const JSONRS_ARRAY: i32 = 6;
const JSONRS_OBJECT: i32 = 7;

pub enum JsonValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
    Raw(Vec<u8>),
}

impl From<SerdeValue> for JsonValue {
    fn from(value: SerdeValue) -> Self {
        match value {
            SerdeValue::Null => Self::Null,
            SerdeValue::Bool(value) => Self::Bool(value),
            SerdeValue::Number(value) => Self::Number(value),
            SerdeValue::String(value) => Self::String(value),
            SerdeValue::Array(value) => {
                Self::Array(value.into_iter().map(JsonValue::from).collect())
            }
            SerdeValue::Object(value) => Self::Object(
                value
                    .into_iter()
                    .map(|(key, value)| (key, JsonValue::from(value)))
                    .collect(),
            ),
        }
    }
}

fn write_json(value: &JsonValue, output: &mut Vec<u8>) -> serde_json::Result<()> {
    match value {
        JsonValue::Null => output.extend_from_slice(b"null"),
        JsonValue::Bool(value) => {
            output.extend_from_slice(if *value { b"true" } else { b"false" });
        }
        JsonValue::Number(value) => output.extend_from_slice(value.to_string().as_bytes()),
        JsonValue::String(value) => serde_json::to_writer(&mut *output, value)?,
        JsonValue::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                write_json(value, output)?;
            }
            output.push(b']');
        }
        JsonValue::Object(values) => {
            output.push(b'{');
            for (index, (key, value)) in values.iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                serde_json::to_writer(&mut *output, key)?;
                output.push(b':');
                write_json(value, output)?;
            }
            output.push(b'}');
        }
        JsonValue::Raw(value) => output.extend_from_slice(value),
    }
    Ok(())
}

unsafe fn value_ref<'a>(value: *const JsonValue) -> Option<&'a JsonValue> {
    unsafe { value.as_ref() }
}

unsafe fn value_mut<'a>(value: *mut JsonValue) -> Option<&'a mut JsonValue> {
    unsafe { value.as_mut() }
}

unsafe fn bytes<'a>(data: *const u8, len: usize) -> Result<&'a [u8], &'static str> {
    if data.is_null() && len != 0 {
        return Err("null data pointer");
    }
    Ok(if len == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(data, len) }
    })
}

unsafe fn write_error(error: *mut *mut c_char, message: impl AsRef<str>) {
    if error.is_null() {
        return;
    }
    let message = message.as_ref().replace('\0', "\\0");
    let message = CString::new(message).expect("NUL bytes were replaced");
    unsafe { *error = message.into_raw() };
}

#[unsafe(no_mangle)]
/// # Safety
/// `error` must be null or a pointer returned through this library's error output.
pub unsafe extern "C" fn mruby_jsonrs_error_free(error: *mut c_char) {
    if !error.is_null() {
        unsafe { drop(CString::from_raw(error)) };
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be null or an owned pointer returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_value_free(value: *mut JsonValue) {
    if !value.is_null() {
        unsafe { drop(Box::from_raw(value)) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mruby_jsonrs_null_new() -> *mut JsonValue {
    Box::into_raw(Box::new(JsonValue::Null))
}

#[unsafe(no_mangle)]
pub extern "C" fn mruby_jsonrs_bool_new(value: bool) -> *mut JsonValue {
    Box::into_raw(Box::new(JsonValue::Bool(value)))
}

#[unsafe(no_mangle)]
pub extern "C" fn mruby_jsonrs_i64_new(value: i64) -> *mut JsonValue {
    Box::into_raw(Box::new(JsonValue::Number(Number::from(value))))
}

#[unsafe(no_mangle)]
/// # Safety
/// `error` must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_f64_new(
    value: f64,
    error: *mut *mut c_char,
) -> *mut JsonValue {
    match Number::from_f64(value) {
        Some(number) => Box::into_raw(Box::new(JsonValue::Number(number))),
        None => {
            unsafe { write_error(error, "non-finite numbers are not valid JSON") };
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `data` must reference `len` readable bytes, and `error` must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_string_new(
    data: *const u8,
    len: usize,
    error: *mut *mut c_char,
) -> *mut JsonValue {
    let data = match unsafe { bytes(data, len) } {
        Ok(data) => data,
        Err(message) => {
            unsafe { write_error(error, message) };
            return ptr::null_mut();
        }
    };
    match std::str::from_utf8(data) {
        Ok(string) => Box::into_raw(Box::new(JsonValue::String(string.to_owned()))),
        Err(err) => {
            unsafe { write_error(error, format!("string is not valid UTF-8: {err}")) };
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `data` must reference `len` readable bytes, and `error` must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_raw_new(
    data: *const u8,
    len: usize,
    error: *mut *mut c_char,
) -> *mut JsonValue {
    match unsafe { bytes(data, len) } {
        Ok(data) => Box::into_raw(Box::new(JsonValue::Raw(data.to_vec()))),
        Err(message) => {
            unsafe { write_error(error, message) };
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mruby_jsonrs_array_new() -> *mut JsonValue {
    Box::into_raw(Box::new(JsonValue::Array(Vec::new())))
}

#[unsafe(no_mangle)]
/// # Safety
/// Both pointers must be owned values returned by this library. On success,
/// ownership of `child` is transferred to `array`.
pub unsafe extern "C" fn mruby_jsonrs_array_push(
    array: *mut JsonValue,
    child: *mut JsonValue,
) -> bool {
    let Some(JsonValue::Array(array)) = (unsafe { value_mut(array) }) else {
        return false;
    };
    if child.is_null() {
        return false;
    }
    array.push(unsafe { *Box::from_raw(child) });
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn mruby_jsonrs_object_new() -> *mut JsonValue {
    Box::into_raw(Box::new(JsonValue::Object(BTreeMap::new())))
}

#[unsafe(no_mangle)]
/// # Safety
/// `object` and `child` must be owned values returned by this library, `key`
/// must reference `key_len` readable bytes, and `error` must be null or writable.
/// On success, ownership of `child` is transferred to `object`.
pub unsafe extern "C" fn mruby_jsonrs_object_insert(
    object: *mut JsonValue,
    key: *const u8,
    key_len: usize,
    child: *mut JsonValue,
    error: *mut *mut c_char,
) -> bool {
    let Some(JsonValue::Object(object)) = (unsafe { value_mut(object) }) else {
        unsafe { write_error(error, "invalid JSON object") };
        return false;
    };
    if child.is_null() {
        unsafe { write_error(error, "invalid JSON object value") };
        return false;
    }
    let key = match unsafe { bytes(key, key_len) }
        .and_then(|key| std::str::from_utf8(key).map_err(|_| "object key is not valid UTF-8"))
    {
        Ok(key) => key.to_owned(),
        Err(message) => {
            unsafe { write_error(error, message) };
            return false;
        }
    };
    object.insert(key, unsafe { *Box::from_raw(child) });
    true
}

#[unsafe(no_mangle)]
/// # Safety
/// `data` must reference `len` readable bytes, and `error` must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_parse(
    data: *const u8,
    len: usize,
    error: *mut *mut c_char,
) -> *mut JsonValue {
    let data = match unsafe { bytes(data, len) } {
        Ok(data) => data,
        Err(message) => {
            unsafe { write_error(error, message) };
            return ptr::null_mut();
        }
    };
    match serde_json::from_slice::<SerdeValue>(data) {
        Ok(value) => Box::into_raw(Box::new(JsonValue::from(value))),
        Err(err) => {
            unsafe { write_error(error, err.to_string()) };
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be a live value returned by this library. `output` and
/// `output_len` must be writable, and `error` must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_generate(
    value: *const JsonValue,
    output: *mut *mut u8,
    output_len: *mut usize,
    error: *mut *mut c_char,
) -> bool {
    let Some(value) = (unsafe { value_ref(value) }) else {
        unsafe { write_error(error, "invalid JSON value") };
        return false;
    };
    if output.is_null() || output_len.is_null() {
        unsafe { write_error(error, "invalid output pointer") };
        return false;
    }

    let mut bytes = Vec::new();
    if let Err(err) = write_json(value, &mut bytes) {
        unsafe { write_error(error, err.to_string()) };
        return false;
    }
    let mut bytes = bytes.into_boxed_slice();
    unsafe {
        *output_len = bytes.len();
        *output = bytes.as_mut_ptr();
    }
    std::mem::forget(bytes);
    true
}

#[unsafe(no_mangle)]
/// # Safety
/// `data` and `len` must be the exact pair returned by `mruby_jsonrs_generate`.
pub unsafe extern "C" fn mruby_jsonrs_bytes_free(data: *mut u8, len: usize) {
    if !data.is_null() {
        let slice = ptr::slice_from_raw_parts_mut(data, len);
        unsafe { drop(Box::from_raw(slice)) };
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be a live value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_value_type(value: *const JsonValue) -> i32 {
    match unsafe { value_ref(value) } {
        Some(JsonValue::Null) => JSONRS_NULL,
        Some(JsonValue::Bool(_)) => JSONRS_BOOL,
        Some(JsonValue::Number(number)) if number.is_i64() => JSONRS_I64,
        Some(JsonValue::Number(number)) if number.is_u64() => JSONRS_U64,
        Some(JsonValue::Number(_)) => JSONRS_F64,
        Some(JsonValue::String(_)) => JSONRS_STRING,
        Some(JsonValue::Array(_)) => JSONRS_ARRAY,
        Some(JsonValue::Object(_)) => JSONRS_OBJECT,
        Some(JsonValue::Raw(_)) | None => -1,
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be a live boolean value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_bool_get(value: *const JsonValue) -> bool {
    matches!(unsafe { value_ref(value) }, Some(JsonValue::Bool(true)))
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be a live signed integer value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_i64_get(value: *const JsonValue) -> i64 {
    match unsafe { value_ref(value) } {
        Some(JsonValue::Number(value)) => value.as_i64().unwrap_or_default(),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be a live unsigned integer value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_u64_get(value: *const JsonValue) -> u64 {
    match unsafe { value_ref(value) } {
        Some(JsonValue::Number(value)) => value.as_u64().unwrap_or_default(),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be a live floating-point value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_f64_get(value: *const JsonValue) -> f64 {
    match unsafe { value_ref(value) } {
        Some(JsonValue::Number(value)) => value.as_f64().unwrap_or_default(),
        _ => 0.0,
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be a live string value returned by this library, and `len`
/// must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_string_data(
    value: *const JsonValue,
    len: *mut usize,
) -> *const u8 {
    let Some(JsonValue::String(value)) = (unsafe { value_ref(value) }) else {
        return ptr::null();
    };
    if !len.is_null() {
        unsafe { *len = value.len() };
    }
    value.as_ptr()
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be a live array value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_array_len(value: *const JsonValue) -> usize {
    match unsafe { value_ref(value) } {
        Some(JsonValue::Array(value)) => value.len(),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be a live array value returned by this library. The returned
/// pointer is borrowed from `value` and must not outlive it.
pub unsafe extern "C" fn mruby_jsonrs_array_get(
    value: *const JsonValue,
    index: usize,
) -> *const JsonValue {
    match unsafe { value_ref(value) } {
        Some(JsonValue::Array(value)) => value.get(index).map_or(ptr::null(), |value| value),
        _ => ptr::null(),
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be a live object value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_object_len(value: *const JsonValue) -> usize {
    match unsafe { value_ref(value) } {
        Some(JsonValue::Object(value)) => value.len(),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be a live object value returned by this library, `index` must
/// be in bounds, and `len` must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_object_key(
    value: *const JsonValue,
    index: usize,
    len: *mut usize,
) -> *const u8 {
    let Some(JsonValue::Object(value)) = (unsafe { value_ref(value) }) else {
        return ptr::null();
    };
    let Some((key, _)) = value.iter().nth(index) else {
        return ptr::null();
    };
    if !len.is_null() {
        unsafe { *len = key.len() };
    }
    key.as_ptr()
}

#[unsafe(no_mangle)]
/// # Safety
/// `value` must be a live object value returned by this library. The returned
/// pointer is borrowed from `value` and must not outlive it.
pub unsafe extern "C" fn mruby_jsonrs_object_value(
    value: *const JsonValue,
    index: usize,
) -> *const JsonValue {
    match unsafe { value_ref(value) } {
        Some(JsonValue::Object(value)) => value
            .iter()
            .nth(index)
            .map_or(ptr::null(), |(_, value)| value),
        _ => ptr::null(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn parses_and_generates_nested_json() {
        let source = br#"{"items":[1,true,null],"name":"jsonrs"}"#;
        let mut error = ptr::null_mut();
        let value = unsafe { mruby_jsonrs_parse(source.as_ptr(), source.len(), &mut error) };
        assert!(!value.is_null());
        assert!(error.is_null());

        let mut output = ptr::null_mut();
        let mut output_len = 0;
        assert!(unsafe { mruby_jsonrs_generate(value, &mut output, &mut output_len, &mut error) });
        let generated = unsafe { slice::from_raw_parts(output, output_len) };
        assert_eq!(generated, source);

        unsafe {
            mruby_jsonrs_bytes_free(output, output_len);
            mruby_jsonrs_value_free(value);
        }
    }

    #[test]
    fn preserves_raw_json_fragments() {
        let raw = br#"{ "raw" : true }"#;
        let mut error = ptr::null_mut();
        let value = mruby_jsonrs_array_new();
        let child = unsafe { mruby_jsonrs_raw_new(raw.as_ptr(), raw.len(), &mut error) };
        assert!(unsafe { mruby_jsonrs_array_push(value, child) });

        let mut output = ptr::null_mut();
        let mut output_len = 0;
        assert!(unsafe { mruby_jsonrs_generate(value, &mut output, &mut output_len, &mut error) });
        let generated = unsafe { slice::from_raw_parts(output, output_len) };
        assert_eq!(generated, br#"[{ "raw" : true }]"#);

        unsafe {
            mruby_jsonrs_bytes_free(output, output_len);
            mruby_jsonrs_value_free(value);
        }
    }

    #[test]
    fn reports_parse_errors() {
        let source = b"{";
        let mut error = ptr::null_mut();
        let value = unsafe { mruby_jsonrs_parse(source.as_ptr(), source.len(), &mut error) };
        assert!(value.is_null());
        assert!(!error.is_null());
        let message = unsafe { CStr::from_ptr(error) }.to_string_lossy();
        assert!(message.contains("EOF"));
        unsafe { mruby_jsonrs_error_free(error) };
    }
}
