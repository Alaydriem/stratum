// Bytes that the owner keeps alive for the stated lifetime; ptr may be null when len is 0
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StBytes {
    pub ptr: *const u8,
    pub len: usize,
}

impl StBytes {
    pub fn empty() -> Self {
        Self { ptr: std::ptr::null(), len: 0 }
    }

    // A view of `bytes`, valid while `bytes` is alive and unchanged.
    pub fn of(bytes: &[u8]) -> Self {
        Self { ptr: bytes.as_ptr(), len: bytes.len() }
    }
}
