//! JNI surface used by the Kotlin app.
//!
//! Every entry point returns a JSON string. Keeping the ABI to a handful of
//! string-returning functions makes the Kotlin side small and lets the Rust
//! core evolve without regenerating headers.

use crate::asm::assemble;
use crate::bench::{benchmark, benchmarks};
use crate::cpu::cpu_info;
use crate::disasm::{disassemble, hex_dump};
use crate::exec::{disassemble_hex, parse_hex, run_raw, run_source};
use crate::lexer::HighlightSession;
use crate::syntax::{CONDITIONS, DIRECTIVES, INSTRUCTIONS, REGISTER_NAMES};
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;
use serde_json::{json, Value};

const VERSION: &str = "0.1.0";

fn read_string(env: &mut JNIEnv, s: &JString) -> Result<String, String> {
    env.get_string(s)
        .map(|v| v.into())
        .map_err(|e| format!("could not read Java string: {e}"))
}

fn json_string(env: &mut JNIEnv, value: Value) -> jstring {
    let text = serde_json::to_string(&value).unwrap_or_else(|_| "{\"ok\":false,\"error\":\"serialization failed\"}".into());
    match env.new_string(text) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn err_json(env: &mut JNIEnv, message: &str) -> jstring {
    json_string(env, json!({ "ok": false, "error": message }))
}

#[no_mangle]
pub extern "system" fn Java_dev_a64studio_core_NativeBridge_nativeVersion(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    json_string(&mut env, json!({ "version": VERSION, "name": "A64 Studio core" }))
}

#[no_mangle]
pub extern "system" fn Java_dev_a64studio_core_NativeBridge_nativeAssemble(
    mut env: JNIEnv,
    _class: JClass,
    source: JString,
) -> jstring {
    let source = match read_string(&mut env, &source) {
        Ok(s) => s,
        Err(e) => return err_json(&mut env, &e),
    };
    let r = assemble(&source);
    json_string(&mut env, serde_json::to_value(r).unwrap_or_else(|_| json!({ "ok": false })))
}

#[no_mangle]
pub extern "system" fn Java_dev_a64studio_core_NativeBridge_nativeLex(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
) -> jstring {
    let text = match read_string(&mut env, &text) {
        Ok(s) => s,
        Err(e) => return err_json(&mut env, &e),
    };
    let mut session = HighlightSession::new();
    session.set_text(&text);
    let n = session.line_count();
    let tokens = session.token_string(1, n);
    json_string(&mut env, json!({ "ok": true, "lines": n, "tokens": tokens }))
}

#[no_mangle]
pub extern "system" fn Java_dev_a64studio_core_NativeBridge_nativeDisassemble(
    mut env: JNIEnv,
    _class: JClass,
    hex: JString,
    base: jni::sys::jlong,
) -> jstring {
    let hex = match read_string(&mut env, &hex) {
        Ok(s) => s,
        Err(e) => return err_json(&mut env, &e),
    };
    match disassemble_hex(&hex, base as u64) {
        Ok(insns) => json_string(&mut env, json!({ "ok": true, "instructions": insns })),
        Err(e) => err_json(&mut env, &e),
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "system" fn Java_dev_a64studio_core_NativeBridge_nativeDisassembleBytes(
    mut env: JNIEnv,
    _class: JClass,
    bytes: jni::sys::jbyteArray,
    base: jni::sys::jlong,
) -> jstring {
    let arr = unsafe { jni::objects::JByteArray::from_raw(bytes) };
    let data = match env.convert_byte_array(&arr) {
        Ok(v) => v,
        Err(e) => return err_json(&mut env, &format!("could not read byte array: {e}")),
    };
    let insns = disassemble(&data, base as u64);
    json_string(&mut env, json!({ "ok": true, "instructions": insns }))
}

#[no_mangle]
pub extern "system" fn Java_dev_a64studio_core_NativeBridge_nativeRun(
    mut env: JNIEnv,
    _class: JClass,
    source: JString,
    timeout_ms: jni::sys::jlong,
) -> jstring {
    let source = match read_string(&mut env, &source) {
        Ok(s) => s,
        Err(e) => return err_json(&mut env, &e),
    };
    let r = run_source(&source, timeout_ms.max(1) as u64);
    json_string(&mut env, serde_json::to_value(r).unwrap_or_else(|_| json!({ "ok": false })))
}

#[no_mangle]
pub extern "system" fn Java_dev_a64studio_core_NativeBridge_nativeRunHex(
    mut env: JNIEnv,
    _class: JClass,
    hex: JString,
    entry: jni::sys::jlong,
    timeout_ms: jni::sys::jlong,
) -> jstring {
    let hex = match read_string(&mut env, &hex) {
        Ok(s) => s,
        Err(e) => return err_json(&mut env, &e),
    };
    let bytes = match parse_hex(&hex) {
        Ok(b) => b,
        Err(e) => return err_json(&mut env, &e),
    };
    let r = run_raw(bytes, entry.max(0) as u64, timeout_ms.max(1) as u64);
    json_string(&mut env, serde_json::to_value(r).unwrap_or_else(|_| json!({ "ok": false })))
}

#[no_mangle]
pub extern "system" fn Java_dev_a64studio_core_NativeBridge_nativeCpuInfo(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    json_string(&mut env, serde_json::to_value(cpu_info()).unwrap_or_else(|_| json!({ "ok": false })))
}

#[no_mangle]
pub extern "system" fn Java_dev_a64studio_core_NativeBridge_nativeReference(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let conditions: Vec<Value> = CONDITIONS
        .iter()
        .map(|(name, code)| json!({ "name": name, "code": code }))
        .collect();
    let registers: Vec<&str> = REGISTER_NAMES.to_vec();
    let directives: Vec<&str> = DIRECTIVES.to_vec();
    json_string(
        &mut env,
        json!({
            "ok": true,
            "instructions": INSTRUCTIONS,
            "conditions": conditions,
            "registers": registers,
            "directives": directives,
        }),
    )
}

#[no_mangle]
pub extern "system" fn Java_dev_a64studio_core_NativeBridge_nativeBenchmarkList(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    json_string(&mut env, json!({ "ok": true, "benchmarks": benchmarks() }))
}

#[no_mangle]
pub extern "system" fn Java_dev_a64studio_core_NativeBridge_nativeRunBenchmark(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
    timeout_ms: jni::sys::jlong,
) -> jstring {
    let id = match read_string(&mut env, &id) {
        Ok(s) => s,
        Err(e) => return err_json(&mut env, &e),
    };
    let r = benchmark(&id, timeout_ms.max(1) as u64);
    json_string(&mut env, serde_json::to_value(r).unwrap_or_else(|_| json!({ "ok": false })))
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "system" fn Java_dev_a64studio_core_NativeBridge_nativeHexDump(
    mut env: JNIEnv,
    _class: JClass,
    bytes: jni::sys::jbyteArray,
    base: jni::sys::jlong,
) -> jstring {
    let arr = unsafe { jni::objects::JByteArray::from_raw(bytes) };
    let data = match env.convert_byte_array(&arr) {
        Ok(v) => v,
        Err(e) => return err_json(&mut env, &format!("could not read byte array: {e}")),
    };
    json_string(&mut env, json!({ "ok": true, "text": hex_dump(&data, base as u64) }))
}

/// Kept for the Kotlin layer: JSON shape documentation lives in one place.
pub fn schema_version() -> &'static str {
    "1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_stable() {
        assert_eq!(schema_version(), "1");
    }
}
