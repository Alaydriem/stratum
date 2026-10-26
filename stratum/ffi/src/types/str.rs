// UTF-8 bytes that the caller owns for the duration of the call; ptr may be null when len is 0
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StStr {
    pub ptr: *const u8,
    pub len: usize,
}
