// Shared build-script helper (pulled in with `include!` by each binary's build.rs):
// compiles a Windows resource file with the app icon and version info using the
// Windows SDK `rc.exe` and links the resulting .res into the package's binaries.
// Missing rc.exe is a warning, not an error: the binaries just keep the default icon.

#[allow(dead_code)]
fn embed_app_resources(file_description: &str, original_filename: &str) {
    use std::path::{Path, PathBuf};
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let icon = manifest.join("../../packaging/assets/app.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    println!("cargo:rerun-if-changed=../../packaging/winres.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let Some(rc) = find_rc() else {
        println!("cargo:warning=rc.exe not found; building without the app icon");
        return;
    };
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let mut parts: Vec<u16> = version.split(['.', '-']).filter_map(|p| p.parse().ok()).collect();
    parts.resize(4, 0);
    let commas = parts.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",");
    let dotted = parts.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(".");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let rc_file = out.join("app.rc");
    let res_file = out.join("app.res");
    let icon_path = std::fs::canonicalize(&icon).unwrap_or(icon).display().to_string();
    let icon_path = icon_path.trim_start_matches(r"\\?\").replace('\\', "\\\\");
    let script = format!(
        r#"#pragma code_page(65001)
1 ICON "{icon_path}"
1 VERSIONINFO
FILEVERSION {commas}
PRODUCTVERSION {commas}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "CompanyName", "AttendFlow"
      VALUE "FileDescription", "{file_description}"
      VALUE "FileVersion", "{dotted}"
      VALUE "ProductName", "AttendFlow Desktop"
      VALUE "ProductVersion", "{dotted}"
      VALUE "OriginalFilename", "{original_filename}"
      VALUE "LegalCopyright", "Copyright (C) AttendFlow"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    );
    std::fs::write(&rc_file, script).unwrap();
    let status = std::process::Command::new(&rc)
        .args(["/nologo", "/fo"])
        .arg(&res_file)
        .arg(&rc_file)
        .status();
    match status {
        Ok(s) if s.success() => println!("cargo:rustc-link-arg-bins={}", res_file.display()),
        other => println!("cargo:warning=rc.exe failed ({other:?}); building without the app icon"),
    }

    fn find_rc() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("RC") {
            return Some(PathBuf::from(p));
        }
        let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
            Ok("aarch64") => "arm64",
            Ok("x86") => "x86",
            _ => "x64",
        };
        let kits = std::env::var_os("ProgramFiles(x86)")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"))
            .join(r"Windows Kits\10\bin");
        let mut versions: Vec<PathBuf> = std::fs::read_dir(&kits)
            .ok()?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.join(arch).join("rc.exe").is_file())
            .collect();
        // Version folders are "10.0.N.0"; compare numerically.
        let key = |p: &Path| -> Vec<u32> {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.split('.').filter_map(|x| x.parse().ok()).collect())
                .unwrap_or_default()
        };
        versions.sort_by_key(|p| key(p));
        versions.pop().map(|v| v.join(arch).join("rc.exe"))
    }
}
