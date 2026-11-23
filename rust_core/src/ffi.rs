//! FFI 桥接层：C ABI 接口，供 Flutter (dart:ffi) 调用

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use crate::download::download_file;
use crate::version::{fetch_version_manifest, fetch_announcement};

/// 将 Rust 字符串转换为 C 字符串（调用方需用 free_string 释放）
#[no_mangle]
pub extern "C" fn mc_string_from_rust(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(s) };
    match c_str.to_str() {
        Ok(rust_str) => match CString::new(rust_str) {
            Ok(c_string) => c_string.into_raw(),
            Err(_) => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// 释放由 mc_string_from_rust 分配的 C 字符串
#[no_mangle]
pub extern "C" fn mc_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// 获取版本清单 JSON 字符串
#[no_mangle]
pub extern "C" fn mc_fetch_version_manifest() -> *mut c_char {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return CString::new("{\"error\":\"failed to create runtime\"}")
            .unwrap()
            .into_raw(),
    };

    let result = rt.block_on(fetch_version_manifest());
    match result {
        Ok(manifest) => {
            let json = serde_json::to_string(&manifest).unwrap_or_default();
            CString::new(json).unwrap().into_raw()
        }
        Err(e) => {
            let json = format!("{{\"error\":\"{}\"}}", e);
            CString::new(json).unwrap().into_raw()
        }
    }
}

/// 下载文件
/// url: 下载地址
/// dest: 目标文件路径
/// 返回：下载字节数（失败返回负数，通过 mc_last_error 获取详情）
#[no_mangle]
pub extern "C" fn mc_download_file(url: *const c_char, dest: *const c_char) -> i64 {
    if url.is_null() || dest.is_null() {
        return -1;
    }

    let url_str = match unsafe { std::ffi::CStr::from_ptr(url) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let dest_str = match unsafe { std::ffi::CStr::from_ptr(dest) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    let result = rt.block_on(download_file(url_str, std::path::Path::new(dest_str)));
    match result {
        Ok(bytes) => bytes as i64,
        Err(_) => -1,
    }
}

/// 获取公告 JSON 字符串
/// cn: 1 使用中文源（Gitee），0 使用国际源（GitHub raw）
#[no_mangle]
pub extern "C" fn mc_fetch_announcement(cn: i32) -> *mut c_char {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return CString::new("{\"error\":\"failed to create runtime\"}")
            .unwrap()
            .into_raw(),
    };

    let result = rt.block_on(fetch_announcement(cn != 0));
    match result {
        Ok(announcement) => {
            let json = serde_json::to_string(&announcement).unwrap_or_default();
            CString::new(json).unwrap().into_raw()
        }
        Err(e) => {
            let json = format!("{{\"error\":\"{}\"}}", e);
            CString::new(json).unwrap().into_raw()
        }
    }
}

/// 获取上次错误信息（简化：实际可用全局错误栈）
#[no_mangle]
pub extern "C" fn mc_last_error() -> *mut c_char {
    CString::new("no error").unwrap().into_raw()
}

/// 版本号（供 Dart 显示）
#[no_mangle]
pub extern "C" fn mc_version() -> *mut c_char {
    CString::new(env!("CARGO_PKG_VERSION")).unwrap().into_raw()
}

/// 启动 Minecraft（阻塞调用，返回退出码；实际 UI 中应在 isolate 后台线程调用）
#[no_mangle]
pub extern "C" fn mc_launch(
    version_id: *const c_char,
    game_dir: *const c_char,
    auth_token: *const c_char,
    auth_uuid: *const c_char,
    user_name: *const c_char,
) -> i32 {
    let version_id = c_str_to_rust(version_id);
    let game_dir = c_str_to_rust(game_dir);
    let auth_token = c_str_to_rust(auth_token);
    let auth_uuid = c_str_to_rust(auth_uuid);
    let user_name = c_str_to_rust(user_name);

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    let config = crate::launch::LaunchConfig {
        version_id: version_id.clone(),
        game_dir: game_dir.clone(),
        assets_dir: format!("{}/assets", game_dir),
        asset_index_id: version_id.clone(),
        auth_access_token: auth_token,
        auth_uuid,
        auth_user_type: user_name,
        ..Default::default()
    };

    match rt.block_on(crate::launch::launch_minecraft(config)) {
        Ok(mut child) => match rt.block_on(child.wait()) {
            Ok(status) => status.code().unwrap_or(-1),
            Err(_) => -2,
        },
        Err(_) => -3,
    }
}

fn c_str_to_rust(s: *const c_char) -> String {
    if s.is_null() {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(s) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_str_conversion() {
        let rust = CString::new("hello").unwrap();
        let ptr = rust.into_raw();
        let back = c_str_to_rust(ptr);
        assert_eq!(back, "hello");
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}
