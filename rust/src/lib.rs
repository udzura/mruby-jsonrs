use serde_json::{Map, Number, Value};
use std::ffi::{c_char, CString};
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

unsafe fn value_ref<'a>(value: *const Value) -> Option<&'a Value> {
    unsafe { value.as_ref() }
}

unsafe fn value_mut<'a>(value: *mut Value) -> Option<&'a mut Value> {
    unsafe { value.as_mut() }
}

unsafe fn bytes<'a>(data: *const u8, len: usize) -> Result<&'a [u8], &'static str> {
    if data.is_null() && len != 0 {
        return Err("null data pointer");
    }
    Ok(if len == 0 {
        &[]
    } else {
        slice::from_raw_parts(data, len)
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

#[no_mangle]
/// # Safety
/// `error` must be null or a pointer returned through this library's error output.
pub unsafe extern "C" fn mruby_jsonrs_error_free(error: *mut c_char) {
    if !error.is_null() {
        unsafe { drop(CString::from_raw(error)) };
    }
}

#[no_mangle]
/// # Safety
/// `value` must be null or an owned pointer returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_value_free(value: *mut Value) {
    if !value.is_null() {
        unsafe { drop(Box::from_raw(value)) };
    }
}

#[no_mangle]
pub extern "C" fn mruby_jsonrs_null_new() -> *mut Value {
    Box::into_raw(Box::new(Value::Null))
}

#[no_mangle]
pub extern "C" fn mruby_jsonrs_bool_new(value: bool) -> *mut Value {
    Box::into_raw(Box::new(Value::Bool(value)))
}

#[no_mangle]
pub extern "C" fn mruby_jsonrs_i64_new(value: i64) -> *mut Value {
    Box::into_raw(Box::new(Value::Number(Number::from(value))))
}

#[no_mangle]
/// # Safety
/// `error` must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_f64_new(value: f64, error: *mut *mut c_char) -> *mut Value {
    match Number::from_f64(value) {
        Some(number) => Box::into_raw(Box::new(Value::Number(number))),
        None => {
            write_error(error, "non-finite numbers are not valid JSON");
            ptr::null_mut()
        }
    }
}

#[no_mangle]
/// # Safety
/// `data` must reference `len` readable bytes, and `error` must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_string_new(
    data: *const u8,
    len: usize,
    error: *mut *mut c_char,
) -> *mut Value {
    let data = match unsafe { bytes(data, len) } {
        Ok(data) => data,
        Err(message) => {
            write_error(error, message);
            return ptr::null_mut();
        }
    };
    match std::str::from_utf8(data) {
        Ok(string) => Box::into_raw(Box::new(Value::String(string.to_owned()))),
        Err(err) => {
            write_error(error, format!("string is not valid UTF-8: {err}"));
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn mruby_jsonrs_array_new() -> *mut Value {
    Box::into_raw(Box::new(Value::Array(Vec::new())))
}

#[no_mangle]
/// # Safety
/// Both pointers must be owned values returned by this library. On success,
/// ownership of `child` is transferred to `array`.
pub unsafe extern "C" fn mruby_jsonrs_array_push(array: *mut Value, child: *mut Value) -> bool {
    let Some(Value::Array(array)) = value_mut(array) else {
        return false;
    };
    if child.is_null() {
        return false;
    }
    array.push(unsafe { *Box::from_raw(child) });
    true
}

#[no_mangle]
pub extern "C" fn mruby_jsonrs_object_new() -> *mut Value {
    Box::into_raw(Box::new(Value::Object(Map::new())))
}

#[no_mangle]
/// # Safety
/// `object` and `child` must be owned values returned by this library, `key`
/// must reference `key_len` readable bytes, and `error` must be null or writable.
/// On success, ownership of `child` is transferred to `object`.
pub unsafe extern "C" fn mruby_jsonrs_object_insert(
    object: *mut Value,
    key: *const u8,
    key_len: usize,
    child: *mut Value,
    error: *mut *mut c_char,
) -> bool {
    let Some(Value::Object(object)) = value_mut(object) else {
        write_error(error, "invalid JSON object");
        return false;
    };
    if child.is_null() {
        write_error(error, "invalid JSON object value");
        return false;
    }
    let key = match unsafe { bytes(key, key_len) }
        .and_then(|key| std::str::from_utf8(key).map_err(|_| "object key is not valid UTF-8"))
    {
        Ok(key) => key.to_owned(),
        Err(message) => {
            write_error(error, message);
            return false;
        }
    };
    object.insert(key, unsafe { *Box::from_raw(child) });
    true
}

#[no_mangle]
/// # Safety
/// `data` must reference `len` readable bytes, and `error` must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_parse(
    data: *const u8,
    len: usize,
    error: *mut *mut c_char,
) -> *mut Value {
    let data = match unsafe { bytes(data, len) } {
        Ok(data) => data,
        Err(message) => {
            write_error(error, message);
            return ptr::null_mut();
        }
    };
    match serde_json::from_slice(data) {
        Ok(value) => Box::into_raw(Box::new(value)),
        Err(err) => {
            write_error(error, err.to_string());
            ptr::null_mut()
        }
    }
}

#[no_mangle]
/// # Safety
/// `value` must be a live value returned by this library. `output` and
/// `output_len` must be writable, and `error` must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_generate(
    value: *const Value,
    output: *mut *mut u8,
    output_len: *mut usize,
    error: *mut *mut c_char,
) -> bool {
    let Some(value) = value_ref(value) else {
        write_error(error, "invalid JSON value");
        return false;
    };
    if output.is_null() || output_len.is_null() {
        write_error(error, "invalid output pointer");
        return false;
    }
    match serde_json::to_vec(value) {
        Ok(bytes) => {
            let mut bytes = bytes.into_boxed_slice();
            unsafe {
                *output_len = bytes.len();
                *output = bytes.as_mut_ptr();
            }
            std::mem::forget(bytes);
            true
        }
        Err(err) => {
            write_error(error, err.to_string());
            false
        }
    }
}

#[no_mangle]
/// # Safety
/// `data` and `len` must be the exact pair returned by `mruby_jsonrs_generate`.
pub unsafe extern "C" fn mruby_jsonrs_bytes_free(data: *mut u8, len: usize) {
    if !data.is_null() {
        let slice = ptr::slice_from_raw_parts_mut(data, len);
        unsafe { drop(Box::from_raw(slice)) };
    }
}

#[no_mangle]
/// # Safety
/// `value` must be a live value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_value_type(value: *const Value) -> i32 {
    match value_ref(value) {
        Some(Value::Null) => JSONRS_NULL,
        Some(Value::Bool(_)) => JSONRS_BOOL,
        Some(Value::Number(number)) if number.is_i64() => JSONRS_I64,
        Some(Value::Number(number)) if number.is_u64() => JSONRS_U64,
        Some(Value::Number(_)) => JSONRS_F64,
        Some(Value::String(_)) => JSONRS_STRING,
        Some(Value::Array(_)) => JSONRS_ARRAY,
        Some(Value::Object(_)) => JSONRS_OBJECT,
        None => -1,
    }
}

#[no_mangle]
/// # Safety
/// `value` must be a live boolean value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_bool_get(value: *const Value) -> bool {
    matches!(value_ref(value), Some(Value::Bool(true)))
}

#[no_mangle]
/// # Safety
/// `value` must be a live signed integer value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_i64_get(value: *const Value) -> i64 {
    value_ref(value).and_then(Value::as_i64).unwrap_or_default()
}

#[no_mangle]
/// # Safety
/// `value` must be a live unsigned integer value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_u64_get(value: *const Value) -> u64 {
    value_ref(value).and_then(Value::as_u64).unwrap_or_default()
}

#[no_mangle]
/// # Safety
/// `value` must be a live floating-point value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_f64_get(value: *const Value) -> f64 {
    value_ref(value).and_then(Value::as_f64).unwrap_or_default()
}

#[no_mangle]
/// # Safety
/// `value` must be a live string value returned by this library, and `len`
/// must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_string_data(
    value: *const Value,
    len: *mut usize,
) -> *const u8 {
    let Some(Value::String(value)) = value_ref(value) else {
        return ptr::null();
    };
    if !len.is_null() {
        unsafe { *len = value.len() };
    }
    value.as_ptr()
}

#[no_mangle]
/// # Safety
/// `value` must be a live array value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_array_len(value: *const Value) -> usize {
    value_ref(value)
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

#[no_mangle]
/// # Safety
/// `value` must be a live array value returned by this library. The returned
/// pointer is borrowed from `value` and must not outlive it.
pub unsafe extern "C" fn mruby_jsonrs_array_get(value: *const Value, index: usize) -> *const Value {
    value_ref(value)
        .and_then(Value::as_array)
        .and_then(|array| array.get(index))
        .map_or(ptr::null(), |value| value)
}

#[no_mangle]
/// # Safety
/// `value` must be a live object value returned by this library.
pub unsafe extern "C" fn mruby_jsonrs_object_len(value: *const Value) -> usize {
    value_ref(value)
        .and_then(Value::as_object)
        .map_or(0, Map::len)
}

#[no_mangle]
/// # Safety
/// `value` must be a live object value returned by this library, `index` must
/// be in bounds, and `len` must be null or writable.
pub unsafe extern "C" fn mruby_jsonrs_object_key(
    value: *const Value,
    index: usize,
    len: *mut usize,
) -> *const u8 {
    let Some((key, _)) = value_ref(value)
        .and_then(Value::as_object)
        .and_then(|object| object.iter().nth(index))
    else {
        return ptr::null();
    };
    if !len.is_null() {
        unsafe { *len = key.len() };
    }
    key.as_ptr()
}

#[no_mangle]
/// # Safety
/// `value` must be a live object value returned by this library. The returned
/// pointer is borrowed from `value` and must not outlive it.
pub unsafe extern "C" fn mruby_jsonrs_object_value(
    value: *const Value,
    index: usize,
) -> *const Value {
    value_ref(value)
        .and_then(Value::as_object)
        .and_then(|object| object.iter().nth(index))
        .map_or(ptr::null(), |(_, value)| value)
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
