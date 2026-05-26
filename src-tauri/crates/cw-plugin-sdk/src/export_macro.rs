#[macro_export]
macro_rules! export_plugin {
    ($tool_type:ty) => {
        #[no_mangle]
        pub extern "C" fn cw_plugin_version() -> u32 {
            $crate::SDK_VERSION
        }

        #[no_mangle]
        pub extern "C" fn cw_plugin_new() -> *mut std::ffi::c_void {
            let boxed: Box<$tool_type> = Box::new(<$tool_type>::default());
            Box::into_raw(boxed) as *mut std::ffi::c_void
        }

        #[no_mangle]
        pub unsafe extern "C" fn cw_plugin_manifest(
            handle: *mut std::ffi::c_void,
        ) -> *const std::ffi::c_char {
            if handle.is_null() {
                return std::ptr::null();
            }
            let tool = &*(handle as *const $tool_type);
            let manifest = tool.manifest();
            match serde_json::to_string(&manifest) {
                Ok(json) => std::ffi::CString::new(json)
                    .map(|c| c.into_raw() as *const std::ffi::c_char)
                    .unwrap_or(std::ptr::null()),
                Err(_) => std::ptr::null(),
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn cw_plugin_execute(
            handle: *mut std::ffi::c_void,
            input_json: *const std::ffi::c_char,
        ) -> *const std::ffi::c_char {
            if handle.is_null() || input_json.is_null() {
                return std::ptr::null();
            }
            let tool = &*(handle as *const $tool_type);
            let input_cstr = std::ffi::CStr::from_ptr(input_json);
            let input_str = match input_cstr.to_str() {
                Ok(s) => s,
                Err(e) => {
                    let err = serde_json::json!({"error": format!("invalid utf8: {e}")});
                    return std::ffi::CString::new(err.to_string())
                        .map(|c| c.into_raw() as *const std::ffi::c_char)
                        .unwrap_or(std::ptr::null());
                }
            };
            let input: $crate::ToolInput = match serde_json::from_str(input_str) {
                Ok(i) => i,
                Err(e) => {
                    let err = serde_json::json!({"error": format!("invalid input: {e}")});
                    return std::ffi::CString::new(err.to_string())
                        .map(|c| c.into_raw() as *const std::ffi::c_char)
                        .unwrap_or(std::ptr::null());
                }
            };
            let response = match tool.execute(input) {
                Ok(output) => serde_json::json!({"ok": output}),
                Err(e) => serde_json::json!({"error": e}),
            };
            std::ffi::CString::new(response.to_string())
                .map(|c| c.into_raw() as *const std::ffi::c_char)
                .unwrap_or(std::ptr::null())
        }

        #[no_mangle]
        pub unsafe extern "C" fn cw_plugin_destroy(handle: *mut std::ffi::c_void) {
            if !handle.is_null() {
                let _ = Box::from_raw(handle as *mut $tool_type);
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn cw_string_free(s: *const std::ffi::c_char) {
            if !s.is_null() {
                let _ = std::ffi::CString::from_raw(s as *mut std::ffi::c_char);
            }
        }
    };
}
