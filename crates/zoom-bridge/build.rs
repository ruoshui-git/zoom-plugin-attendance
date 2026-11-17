//! Build script for the C++ adapter.
//!
//! SDK discovery order (first match wins):
//!   1. `ZOOM_PLUGIN_SDK_DIR` env var -> the architecture root of the SDK
//!      (the folder that contains `export_h/`, `lib/`, `bin/`).
//!   2. `<workspace>/vendor/zoom-plugin-sdk/ACTIVE` -> a text file containing
//!      that path (written by `scripts/extract-sdk.ps1`).
//!
//! If no SDK is found the adapter is built in *stub* mode: everything still
//! compiles and the self-tests run, but SDK calls return ZC_ERR_NOT_AVAILABLE.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};


const SDK_HEADER: &str = "ToolSuiteProxyInterface.h";
const SDK_IMPORT_LIB: &str = "zToolSuiteIPCProxy";
const SDK_DLL: &str = "zToolSuiteIPCProxy.dll";

fn main() {
    println!("cargo:rerun-if-env-changed=ZOOM_PLUGIN_SDK_DIR");
    println!("cargo:rerun-if-changed=native");

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest.parent().unwrap().parent().unwrap().to_path_buf();
    let active_file = workspace.join("vendor").join("zoom-plugin-sdk").join("ACTIVE");
    println!("cargo:rerun-if-changed={}", active_file.display());

    let target = env::var("TARGET").unwrap();
    if !target.contains("windows-msvc") {
        panic!("zoom-bridge only supports *-pc-windows-msvc targets (got {target})");
    }
    if !target.starts_with("x86_64") {
        println!("cargo:warning=zoom-bridge is configured and tested for x86_64 only");
    }

    let sdk = find_sdk(&active_file);

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .include(manifest.join("native"))
        .file("native/zc_common.cpp")
        // ToolSuiteProxyPlatform.h keys dllimport + wchar_t zchar_t off WIN32.
        .define("WIN32", None)
        .define("UNICODE", None)
        .define("_UNICODE", None)
        .flag("/EHsc")
        .flag("/utf-8")
        .warnings(true);

    match &sdk {
        Some(root) => {
            let inc = root.join("export_h");
            let lib = root.join("lib");
            let bin = root.join("bin");
            println!("cargo:rerun-if-changed={}", inc.display());
            build.include(&inc).file("native/zc_sdk.cpp").define("ZC_HAVE_SDK", "1");
            build.compile("zc_bridge");

            println!("cargo:rustc-link-search=native={}", lib.display());
            println!("cargo:rustc-link-lib=dylib={SDK_IMPORT_LIB}");
            // Delay-load so the process can start, diagnose and report a
            // missing/incompatible SDK DLL instead of failing at loader time.
            println!("cargo:rustc-link-lib=dylib=delayimp");
            println!("cargo:rustc-link-arg=/DELAYLOAD:{SDK_DLL}");
            println!("cargo:rustc-link-arg=/IGNORE:4199");
            println!("cargo:rustc-cfg=zc_sdk");
            println!("cargo:rustc-env=ZC_SDK_BIN_DIR={}", bin.display());
            println!("cargo:rustc-env=ZC_SDK_ROOT={}", root.display());
            // Exposed to dependents as DEP_ZC_BRIDGE_SDK_BIN / _SDK_DLL.
            println!("cargo:sdk_bin={}", bin.display());
            println!("cargo:sdk_dll={SDK_DLL}");

            // DLLs are NOT copied at build time (cargo's build-dir layout is
            // not stable). At runtime `ensure_sdk_dll_loaded` falls back to
            // ZC_SDK_BIN_DIR; scripts/package.ps1 copies bin/ beside the exe.
        }
        None => {
            println!(
                "cargo:warning=Zoom Plugin SDK not found: building zoom-bridge in STUB mode. \
                 Run scripts/extract-sdk.ps1 or set ZOOM_PLUGIN_SDK_DIR."
            );
            build.file("native/zc_stub.cpp");
            build.compile("zc_bridge");
            println!("cargo:rustc-env=ZC_SDK_BIN_DIR=");
            println!("cargo:rustc-env=ZC_SDK_ROOT=");
        }
    }
    println!("cargo::rustc-check-cfg=cfg(zc_sdk)");
}

fn find_sdk(active_file: &Path) -> Option<PathBuf> {
    let candidate = env::var_os("ZOOM_PLUGIN_SDK_DIR")
        .map(PathBuf::from)
        .or_else(|| fs::read_to_string(active_file).ok().map(|s| PathBuf::from(s.trim())));
    let root = candidate?;
    if root.join("export_h").join(SDK_HEADER).is_file() && root.join("lib").is_dir() {
        Some(root)
    } else {
        println!(
            "cargo:warning=Configured Zoom Plugin SDK path is invalid (expected export_h/{SDK_HEADER}): {}",
            root.display()
        );
        None
    }
}
