// the C ABI entry points take raw pointers by design; every pointer goes through the boundary's checks
// and every unsafe block states its invariant in a SAFETY comment
#![allow(clippy::not_unsafe_ptr_arg_deref, clippy::missing_safety_doc)]

pub mod api;
pub mod services;
pub mod types;
