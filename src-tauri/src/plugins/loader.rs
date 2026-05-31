use std::ffi::{c_char, c_void, CStr};
use std::path::Path;

use cw_plugin_sdk::{SDK_VERSION, ToolInput, ToolManifest, ToolOutput};
use libloading::{Library, Symbol};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Failed to load library: {0}")]
    LoadError(#[from] libloading::Error),
    #[error("Plugin version mismatch: expected {expected}, got {got}")]
    VersionMismatch { expected: u32, got: u32 },
    #[error("Plugin initialization failed")]
    InitFailed,
    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct LoadedPlugin {
    _library: Library,
    handle: *mut c_void,
    pub manifest: ToolManifest,
    execute_fn: unsafe extern "C" fn(*mut c_void, *const c_char) -> *const c_char,
    destroy_fn: unsafe extern "C" fn(*mut c_void),
    string_free_fn: unsafe extern "C" fn(*const c_char),
}

unsafe impl Send for LoadedPlugin {}
unsafe impl Sync for LoadedPlugin {}

impl LoadedPlugin {
    pub fn execute(&self, input: &ToolInput) -> Result<ToolOutput, PluginError> {
        let input_json = serde_json::to_string(input)?;
        let input_cstring = std::ffi::CString::new(input_json)
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        unsafe {
            let result_ptr = (self.execute_fn)(self.handle, input_cstring.as_ptr());
            if result_ptr.is_null() {
                return Err(PluginError::ExecutionFailed("null response".into()));
            }

            let result_cstr = CStr::from_ptr(result_ptr);
            let result_str = result_cstr
                .to_str()
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?
                .to_string();
            (self.string_free_fn)(result_ptr);

            let response: serde_json::Value = serde_json::from_str(&result_str)?;
            if let Some(error) = response.get("error") {
                let error_message = error
                    .as_str()
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| error.to_string());
                return Err(PluginError::ExecutionFailed(error_message));
            }
            let output: ToolOutput = serde_json::from_value(
                response
                    .get("ok")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            )?;
            Ok(output)
        }
    }
}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { (self.destroy_fn)(self.handle) };
            self.handle = std::ptr::null_mut();
        }
    }
}

pub fn load_plugin(path: &Path) -> Result<LoadedPlugin, PluginError> {
    unsafe {
        let lib = Library::new(path)?;

        let version = {
            let version_fn: Symbol<unsafe extern "C" fn() -> u32> = lib.get(b"cw_plugin_version")?;
            (*version_fn)()
        };
        if version != SDK_VERSION {
            return Err(PluginError::VersionMismatch {
                expected: SDK_VERSION,
                got: version,
            });
        }

        let handle = {
            let new_fn: Symbol<unsafe extern "C" fn() -> *mut c_void> =
                lib.get(b"cw_plugin_new")?;
            (*new_fn)()
        };
        if handle.is_null() {
            return Err(PluginError::InitFailed);
        }

        let manifest_ptr = {
            let manifest_fn: Symbol<unsafe extern "C" fn(*mut c_void) -> *const c_char> =
                lib.get(b"cw_plugin_manifest")?;
            (*manifest_fn)(handle)
        };
        if manifest_ptr.is_null() {
            let destroy = {
                let destroy: Symbol<unsafe extern "C" fn(*mut c_void)> =
                    lib.get(b"cw_plugin_destroy")?;
                *destroy
            };
            destroy(handle);
            return Err(PluginError::ExecutionFailed("null manifest".into()));
        }

        let manifest_str = CStr::from_ptr(manifest_ptr)
            .to_str()
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?
            .to_string();
        let string_free = {
            let string_free: Symbol<unsafe extern "C" fn(*const c_char)> =
                lib.get(b"cw_string_free")?;
            *string_free
        };
        string_free(manifest_ptr);
        let manifest: ToolManifest = serde_json::from_str(&manifest_str)?;

        let execute_fn = {
            let execute_fn: Symbol<
                unsafe extern "C" fn(*mut c_void, *const c_char) -> *const c_char,
            > = lib.get(b"cw_plugin_execute")?;
            *execute_fn
        };
        let destroy_fn = {
            let destroy_fn: Symbol<unsafe extern "C" fn(*mut c_void)> =
                lib.get(b"cw_plugin_destroy")?;
            *destroy_fn
        };
        let string_free_fn = {
            let string_free_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
                lib.get(b"cw_string_free")?;
            *string_free_fn
        };

        Ok(LoadedPlugin {
            _library: lib,
            handle,
            manifest,
            execute_fn,
            destroy_fn,
            string_free_fn,
        })
    }
}

pub fn is_plugin_file(path: &Path) -> bool {
    path.extension()
        .map_or(false, |ext| ext == "so" || ext == "dylib" || ext == "dll")
}

pub fn discover_plugins(dir: &Path) -> Vec<LoadedPlugin> {
    let mut plugins = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_plugin_file(&path) {
                match load_plugin(&path) {
                    Ok(plugin) => {
                        tracing::info!(
                            "loaded plugin: {} from {}",
                            plugin.manifest.name,
                            path.display()
                        );
                        plugins.push(plugin);
                    }
                    Err(err) => {
                        tracing::warn!("failed to load plugin {}: {err}", path.display());
                    }
                }
            }
        }
    }
    plugins
}
