# Environment setup and versions

Recorded on 2026-11-17 on the bootstrap machine (Windows 11 Pro 10.0.26300, x64).
Everything runs natively on Windows (no WSL).

## Installed / used

| Component | Version | Notes |
|---|---|---|
| Rust (rustup) | rustc/cargo 1.100.0, `stable-x86_64-pc-windows-msvc` | pre-existing |
| Visual Studio 2022 Build Tools | 17.14.36603.0 | pre-existing, at `C:\Users\Ruoshui\Documents\Dione\bin\build_tools` (non-default path; found via vswhere) |
| MSVC toolset | v143, 14.44.35207 | pre-existing |
| Windows SDK | 10.0.26100.0 | pre-existing |
| **C++ ATL/MFC** (`Microsoft.VisualStudio.Component.VC.ATLMFC`) | 14.44 | **added during bootstrap** — required by the MFC `PSDKTest` sample |
| CMake | VS-bundled (`Common7\IDE\CommonExtensions\Microsoft\CMake`) | not needed by the build; on PATH after `. .\scripts\dev-env.ps1` |
| Git | 2.38.1 | pre-existing |
| Zoom Workplace | 7.2.5 (51704) | pre-existing, `%APPDATA%\Zoom\bin` (SDK needs >= 7.0.2) |
| **Zoom Plugin SDK for Windows** | **7.2.0.3001** | `zoom-plugin-sdk-windows-7.2.0.3001.zip`, SHA256 `C4EF8189D121658B7E21A5F08B4E536FFC9BC5759FFA3C98D9EE06B0920BA850`; `zToolSuiteIPCProxy.dll` Authenticode-signed by Zoom Communications, Inc. |

Rust crates: see `Cargo.lock` (cc 1.x, windows-sys 0.59, ureq 3, sha2 0.10,
base64 0.22, getrandom 0.3, url 2, clap 4, serde/toml).

No second Visual Studio, no vcpkg, no extra CMake, no v142 toolset were installed.

## Windows Defender Controlled Folder Access (CFA)

CFA is enabled on this machine and the Build Tools live under `Documents`.
The VS Installer was blocked (MSI error 1603, Defender event 1123) until its
executables were added as allowed apps (approved by the owner):

```powershell
Add-MpPreference -ControlledFolderAccessAllowedApplications "C:\Program Files (x86)\Microsoft Visual Studio\Installer\setup.exe"
# (+ the other .exe files in that Installer folder)
```

If cargo/MSBuild ever fail writing into a protected folder, keep the repo
outside `Documents` (it is in `C:\Users\Ruoshui\code`, which is not protected).

## Obtaining the SDK (manual, requires Marketplace login)

1. marketplace.zoom.us → Develop → Build App → **General app** (dev app, *not* AttendFlow).
2. Features → Access → **Plugin SDK** → enable → Windows → **Download**.
3. Put the unmodified zip in `vendor\downloads\` and run `.\scripts\extract-sdk.ps1`.
   It extracts to `vendor\zoom-plugin-sdk\<name>\` and writes `vendor\zoom-plugin-sdk\ACTIVE`
   (path of the `x64` tree), which `crates/zoom-bridge/build.rs` reads.
   Override with `$env:ZOOM_PLUGIN_SDK_DIR = '<...>\x64'`.

Package layout (7.2.0.3001): `arm64/ x64/ x86/` each with `export_h/ lib/ bin/`
(x64 and x86 also have `demo/PSDKTest.sln`), plus `version.txt`.

## Marketplace app configuration (dev app)

- App type: General app, user-managed.
- Scope: `plugin_sdk:read:connection_meta`.
- Redirect URL **and** OAuth allow list: `http://127.0.0.1:47811/callback`
  (must match `redirect_uri` in `%APPDATA%\zoom-companion\config.toml` exactly).
- **Use Public Client OAuth** enabled (App Credentials). Use the **public client ID** (not the confidential Client ID) in `config init --client-id`; loopback redirects are only delivered to the public client. Zoom ignores the port of loopback redirects for public clients, so each login binds a fresh ephemeral port (with the confidential Client ID the port is checked and sign-in fails with "Invalid redirect ... (4,700)"). Put `http://127.0.0.1:47811/callback` in the **PKCE OAuth Redirect URL** field. The plain "OAuth redirect URL" field rejects loopback; the app never uses it, but on the Production tab it is required: enter `https://www.attendflow.org/desktop` (also add it to the allow list). No client secret is needed. If the app can't be a public
  client, `ZOOM_COMPANION_CLIENT_SECRET` can be set **in the shell for local dev only**
  (never stored or committed); the token request then uses HTTP Basic auth.

## Building the official sample

`.\scripts\build-demo.ps1 [-Launch]` → MSBuild `Release|x64` with
`/p:PlatformToolset=v143` (the project targets v142/VS2019; the SDK files are not edited).
Output: `vendor\zoom-plugin-sdk\...\x64\bin\PSDKTest.exe` beside the runtime DLLs.
Run it, press **InitZMToolSuite**, paste an access token in **Token**, and leave
Domain empty (defaults to `https://zoom.us`). Press **UninitZMToolSuite** before closing.
Expected warnings: C4244 (int64→uint) and RC2182 (duplicate control id), both in sample code.
