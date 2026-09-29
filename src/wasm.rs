//! C ABI for WebAssembly hosts, mirroring `src/symbol_wasm.cr`, so the same
//! JavaScript (`symbol_browser.js`, `test_wasm.mjs`) can drive either module.
//!
//! Strings cross the boundary as NUL-terminated UTF-8 in linear memory.
//! Every pointer returned by `symbol_alloc`, `symbol_eval` or
//! `symbol_eval_program` must be released with `symbol_free`.

use std::alloc::{Layout, alloc, dealloc};
use std::ffi::{CStr, c_char};
use std::ptr;

use crate::eval::Bindings;
use crate::value::{EvalResult, Value, join};

/// Bytes reserved in front of each buffer to record its size, so that
/// `symbol_free` needs only the pointer. Also the buffer alignment.
const HEADER: usize = 8;

/// Allocate `size` bytes for the host to write into.
#[unsafe(no_mangle)]
pub extern "C" fn symbol_alloc(size: i32) -> *mut u8 {
    let Ok(size) = usize::try_from(size) else { return ptr::null_mut() };
    let Ok(layout) = Layout::from_size_align(size + HEADER, HEADER) else { return ptr::null_mut() };
    // SAFETY: the layout has a non-zero size; the header write stays inside the allocation.
    unsafe {
        let base = alloc(layout);
        if base.is_null() {
            return base;
        }
        base.cast::<usize>().write(layout.size());
        base.add(HEADER)
    }
}

/// Release a buffer from `symbol_alloc`, `symbol_eval` or `symbol_eval_program`. Null is ignored.
///
/// # Safety
/// `ptr` must be null or a pointer returned by one of those functions and not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn symbol_free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: per the contract, `ptr` sits HEADER bytes past an allocation whose size is stored there.
    unsafe {
        let base = ptr.sub(HEADER);
        let size = base.cast::<usize>().read();
        dealloc(base, Layout::from_size_align_unchecked(size, HEADER));
    }
}

/// Evaluate a NUL-terminated expression. Returns a NUL-terminated result,
/// or `"Error: <message>"`.
///
/// # Safety
/// `input` must point to a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn symbol_eval(input: *const c_char) -> *mut u8 {
    // SAFETY: guaranteed by the caller.
    let source = unsafe { CStr::from_ptr(input) }.to_string_lossy();
    respond(crate::eval(&source, &Bindings::new()))
}

/// Like `symbol_eval`, in program mode (`.` separates statements, `=` assigns),
/// with fresh bindings on each call. Not in the Crystal module.
///
/// # Safety
/// `input` must point to a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn symbol_eval_program(input: *const c_char) -> *mut u8 {
    // SAFETY: guaranteed by the caller.
    let source = unsafe { CStr::from_ptr(input) }.to_string_lossy();
    respond(crate::eval_program(&source, &mut Bindings::new()))
}

/// Hosts written for the Crystal module call `_start` to boot its runtime
/// (`symbol_browser.js` does, as does Node's `wasi.start`). Nothing to do here.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn _start() {}

/// The text `symbol_eval` returns (`SYMBOL.format_result` in Crystal):
/// `Display` output, except that a top-level array shows its items with
/// `Display` rather than `inspect`.
pub fn format_result(result: &EvalResult) -> String {
    match result {
        EvalResult::Resolved(Value::Array(items)) => {
            format!("[{}]", join(items.iter().map(Value::to_string)))
        }
        EvalResult::Resolved(value) => value.to_string(),
        other => other.to_string(),
    }
}

fn respond(result: crate::Result<EvalResult>) -> *mut u8 {
    let text = match result {
        Ok(result) => format_result(&result),
        Err(error) => format!("Error: {error}"),
    };
    let Ok(len) = i32::try_from(text.len() + 1) else { return ptr::null_mut() };
    let out = symbol_alloc(len);
    if !out.is_null() {
        // SAFETY: `out` has room for the text plus the terminating NUL.
        unsafe {
            ptr::copy_nonoverlapping(text.as_ptr(), out, text.len());
            out.add(text.len()).write(0);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn call(f: unsafe extern "C" fn(*const c_char) -> *mut u8, source: &str) -> String {
        let input = CString::new(source).unwrap();
        unsafe {
            let out = f(input.as_ptr());
            let text = CStr::from_ptr(out.cast()).to_str().unwrap().to_owned();
            symbol_free(out);
            text
        }
    }

    #[test]
    fn round_trips_through_the_c_abi() {
        assert_eq!(call(symbol_eval, "Σ [1, 2, 3, 4]"), "10");
        assert_eq!(call(symbol_eval, "7 / 2"), "3.5");
        assert_eq!(call(symbol_eval, "[1, \"a\", [2, \"b\"]]"), "[1, a, [2, \"b\"]]");
        assert_eq!(
            call(symbol_eval, "x = 5"),
            "Error: 1:3: Assignment '=' not allowed in expression context"
        );
        assert_eq!(call(symbol_eval_program, "x = 3. y = x * 2. Σ [x, y, 10]"), "19");
    }

    #[test]
    fn alloc_and_free_accept_edge_cases() {
        unsafe {
            symbol_free(ptr::null_mut());
            let p = symbol_alloc(0);
            assert!(!p.is_null());
            symbol_free(p);
        }
        assert!(symbol_alloc(-1).is_null());
    }
}
