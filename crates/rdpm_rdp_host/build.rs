use std::env;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=native/host.h");
    println!("cargo:rerun-if-changed=native/active_x_host.cpp");
    println!("cargo:rerun-if-changed=native/mstscax_import.cpp");
    println!("cargo:rerun-if-env-changed=VCToolsInstallDir");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env != "msvc" {
        println!("cargo:warning=rdpm_rdp_host 仅在 MSVC 工具链下支持完整 ActiveX 宿主");
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("缺少 OUT_DIR 环境变量"));

    // 1. 生成 mstscax.tlh 类型库头文件
    generate_type_library_bindings(&out_dir);

    // 2. 编译 C++ 静态库
    let mut build = native_cpp_build(&out_dir);
    build.file("native/active_x_host.cpp");
    build.compile("rdpm_rdp_host_native");

    // 3. 查找并链接 ATL 静态库
    link_atl_libraries();

    // 4. 链接 Windows 基础库
    for lib in ["ole32", "oleaut32", "user32", "uuid"] {
        println!("cargo:rustc-link-lib={lib}");
    }
}

fn generate_type_library_bindings(out_dir: &Path) {
    let mut importer = native_cpp_build(out_dir);
    importer.file("native/mstscax_import.cpp");
    importer
        .try_compile_intermediates()
        .expect("系统 RDP 类型库导入失败");

    let generated_header = out_dir.join("mstscax.tlh");
    assert!(
        generated_header.is_file(),
        "MSVC #import 未能生成 {}",
        generated_header.display()
    );
}

fn native_cpp_build(out_dir: &Path) -> cc::Build {
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .include("native")
        .include(out_dir)
        .out_dir(out_dir)
        .flag("/EHsc")
        .flag("/std:c++17")
        .flag("/permissive-")
        .flag("/utf-8")
        .flag("/W3")
        .define("UNICODE", None)
        .define("_UNICODE", None);
    build
}

fn link_atl_libraries() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let arch_dir = match target_arch.as_str() {
        "x86_64" => "x64",
        "x86" => "x86",
        "aarch64" => "arm64",
        _ => "x64",
    };

    if let Ok(vc_tools) = env::var("VCToolsInstallDir") {
        let atl_lib_path = PathBuf::from(vc_tools).join("atlmfc").join("lib").join(arch_dir);
        if atl_lib_path.exists() {
            println!("cargo:rustc-link-search=native={}", atl_lib_path.display());
        }
    }

    println!("cargo:rustc-link-lib=static=atls");
}
