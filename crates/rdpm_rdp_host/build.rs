use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=native/host.h");
    println!("cargo:rerun-if-changed=native/active_x_host.cpp");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env != "msvc" {
        println!("cargo:warning=rdpm_rdp_host 仅在 MSVC 工具链下支持完整 ActiveX 宿主");
        return;
    }

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .include("native")
        .file("native/active_x_host.cpp")
        .flag("/EHsc")
        .flag("/std:c++17")
        .flag("/W3")
        .define("UNICODE", None)
        .define("_UNICODE", None);

    build.compile("rdpm_rdp_host_native");

    // 链接系统与 ATL 依赖库
    println!("cargo:rustc-link-lib=static=atls");
    println!("cargo:rustc-link-lib=ole32");
    println!("cargo:rustc-link-lib=oleaut32");
    println!("cargo:rustc-link-lib=user32");
    println!("cargo:rustc-link-lib=uuid");
}
