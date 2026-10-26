use stratum_ffi::api::pause::st_pause_screen_unwanted;

#[test]
fn pause_screen_unwanted_is_false_for_a_null_handle() {
    assert!(!st_pause_screen_unwanted(std::ptr::null_mut()));
}
