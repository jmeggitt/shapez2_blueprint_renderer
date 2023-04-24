//! This module had generated bindings to OpenGL
#![allow(
    clippy::unused_unit,
    clippy::upper_case_acronyms,
    clippy::too_many_arguments,
    clippy::manual_non_exhaustive
)]

// Basically equivalent to C's #include.
include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));

pub use Gles2 as Gl;

// Helper macro for turning a regular string into a C string
#[macro_export]
macro_rules! c_str {
    ($str:expr) => {{
        const CSTR: &'static ::std::ffi::CStr = unsafe {
            let input = concat!($str, "\0");
            ::std::ffi::CStr::from_bytes_with_nul_unchecked(input.as_bytes())
        };

        CSTR
    }};
}
