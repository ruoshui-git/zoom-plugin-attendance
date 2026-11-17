# zoom-companion

Development baseline for a native **Windows x64** companion app (Rust) that talks
to the installed **Zoom Workplace** client through the official **Zoom Plugin
SDK** (IPC). Working name: `zoom-companion`. Separate from AttendFlow (Zoom Apps SDK).

> Start here if you are continuing the work: **[docs/HANDOFF.md](docs/HANDOFF.md)**.

```text
zoom-companion (Rust CLI, later UI)
  └─ zoom-auth    OAuth 2.0 Authorization Code + PKCE, loopback redirect, DPAPI token store
  └─ zoom-bridge  safe Rust API ── extern "C" ABI ── thin C++ adapter (native/*.cpp)
                    └─ zToolSuiteIPCProxy.dll (official SDK, delay-loaded)
                         └─ IPC ── Zoom Workplace
```

## Quick start (fresh PowerShell)

```powershell
.\scripts\setup.ps1 -CheckOnly        # verify prerequisites (setup.ps1 without -CheckOnly installs)
.\scripts\extract-sdk.ps1             # vendor\downloads\zoom-plugin-sdk-windows-*.zip -> vendor\zoom-plugin-sdk\
.\scripts\build-demo.ps1              # official PSDKTest sample (MFC), optional -Launch
.\scripts\build.ps1                   # cargo build --workspace
.\scripts\test.ps1                    # unit tests + interop selftest
.\scripts\run.ps1 doctor              # environment / SDK / DLL / Zoom / auth checks
pwsh -NoProfile -File .\scripts\verify.ps1 [-BuildDemo] [-Connect]   # full checklist
```

First-time auth (interactive, once per machine/user):

```powershell
.\scripts\run.ps1 config init --client-id <Development Client ID>
.\scripts\run.ps1 auth login          # browser sign-in; same user as Zoom Workplace
.\scripts\run.ps1 connect --seconds 20
```

## Layout

| Path | Purpose |
|---|---|
| `crates/zoom-companion` | CLI app (`doctor`, `selftest`, `config`, `auth`, `connect`, `participants`) |
| `crates/zoom-bridge` | FFI crate: `native/zc_bridge.h` C ABI, C++ adapter, safe Rust session API |
| `crates/zoom-auth` | PKCE, loopback listener, token endpoint, DPAPI store |
| `scripts/` | Reproducible PowerShell scripts |
| `docs/` | Setup, architecture, SDK notes, handoff |
| `vendor/` | **git-ignored** SDK archive + extraction (never committed) |

Secrets are never stored in the repo: config (non-secret) is in
`%APPDATA%\zoom-companion\config.toml`, tokens are DPAPI-encrypted in
`%LOCALAPPDATA%\zoom-companion\tokens.dpapi`.
