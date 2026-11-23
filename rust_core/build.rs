fn main() {
    // 在 Android 上链接系统 Vulkan 库（libvulkan.so），供 ash 调用
    // cargo:rerun-if-changed=build.rs
    if std::env::var_os("CARGO_CFG_ANDROID").is_some() {
        println!("cargo:rustc-link-lib=dylib=vulkan");
    }
}
