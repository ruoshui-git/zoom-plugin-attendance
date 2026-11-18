//! Applies the SDK DLL delay-load flag to this binary. `rustc-link-arg` set by
//! zoom-bridge's build script only affects that package's own targets, so the
//! final executable has to request it itself. Metadata comes from
//! zoom-bridge (`links = "zc_bridge"`) as DEP_ZC_BRIDGE_*.
//! Also embeds the app icon + version info (resource id 1, used for the window icon).
include!("../../packaging/winres.rs");

fn main() {
    println!("cargo:rerun-if-env-changed=DEP_ZC_BRIDGE_SDK_DLL");
    println!("cargo:rerun-if-env-changed=ZOOM_COMPANION_BUILTIN_CLIENT_ID");
    if let Ok(dll) = std::env::var("DEP_ZC_BRIDGE_SDK_DLL") {
        println!("cargo:rustc-link-arg-bins=/DELAYLOAD:{dll}");
        // The unit-test build of the bin may not reference SDK imports.
        println!("cargo:rustc-link-arg-bins=/IGNORE:4199");
    }
    embed_app_resources("AttendFlow Desktop (companion)", "zoom-companion.exe");
}
