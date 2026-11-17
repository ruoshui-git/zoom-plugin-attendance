# Architecture and interop decisions

## Choice: hand-written `extern "C"` ABI + thin C++ adapter

| Option | Verdict |
|---|---|
| **Minimal `extern "C"` bridge** (chosen) | The SDK API is C++ with `std::function` callbacks, `std::string` params, SDK-owned `IToolSuiteVector<T>*` and a pure-virtual listener. A C++ adapter absorbs all of that; Rust sees ~12 C functions with POD structs. No codegen, trivial to audit, works with the `cc` crate only. |
| `cxx` crate | Good for shared types, but `std::function`, `std::wstring`, abstract listener classes and SDK-owned pointers would all still need custom C++ shims; adds a codegen layer for no gain at this size. Reconsider only if the surface grows past several dozen functions. |
| `bindgen` on SDK headers | Not workable: headers are C++ classes/namespaces/std types; vtable layout binding from Rust is unsafe and brittle. |
| Rust COM-style vtable calls | Same problem, plus undefined behaviour risk across compilers. |

## Layers

```text
crates/zoom-companion   business logic / CLI / future UI (no unsafe FFI)
crates/zoom-bridge
  src/lib.rs, session.rs   safe API: Session::start/participants/meeting_info/...
  src/events.rs            owned Event/Participant types; callback trampolines
  src/ffi.rs               the ONLY extern "C" declarations (mirror of zc_bridge.h)
  src/dll.rs               explicit LoadLibraryExW probe of the delay-loaded DLL
  native/zc_bridge.h       C ABI contract
  native/zc_sdk.cpp        real adapter (ZC_HAVE_SDK)   | native/zc_stub.cpp (no SDK)
  native/zc_common.cpp     UTF-8/UTF-16 + self-tests (always built)
```

`build.rs` compiles `zc_sdk.cpp` when an SDK is found (else `zc_stub.cpp` with a
cargo warning), links `zToolSuiteIPCProxy.lib` and passes `/DELAYLOAD:zToolSuiteIPCProxy.dll`.

## Rules enforced at the boundary

- **Ownership:** SDK memory never crosses into Rust. The adapter copies every
  `ZMToolSuiteUser` (incl. `strUserName`) and `GetMeetingProperty` string into
  adapter-owned buffers, valid only during the callback; Rust copies again into
  owned `String`s before returning. `GetParticipantsList()` vectors are SDK-owned
  (the sample never frees them) and are never freed or retained by us.
- **Exceptions:** every exported C function is wrapped in `try/catch(...)` →
  status code; listener methods swallow exceptions (an exception must not unwind
  into the SDK either).
- **Panics:** Rust trampolines use `catch_unwind`; a panic drops the event and logs.
- **Strings:** UTF-8 in Rust ↔ UTF-16 (`zchar_t = wchar_t` because `WIN32` is
  defined) in C++. Conversion is strict for input (invalid UTF-8 → `InvalidArg`),
  lenient for SDK output (bad surrogates → U+FFFD). Tests cover Chinese, accents,
  emoji/ZWJ and empty names. `OnIPCConnectStatusChanged` gives a narrow
  `std::string` with undocumented encoding; we treat it as UTF-8, lossy.
- **Errors:** adapter `ZC_*` status → `BridgeError`; SDK async results carry raw
  `ZMToolSuiteProxyErrors` (note **1 = success, 0 = unknown**) → `status::proxy_error_name`.
- **Threads:** one dedicated `zoom-sdk` thread calls `InitZMToolSuite`, owns all
  SDK calls (queries are sent to it over channels) and pumps Win32 messages
  (the sample is MFC and updates UI directly from callbacks, suggesting callbacks
  arrive on the initialising thread's message loop). **Verified 2026-11-17:**
  `OnAuthResult`/`OnIPCConnectStatusChanged` ran on the `zoom-sdk` thread
  (same tid as `InitZMToolSuite`). Every event carries its Win32 `thread_id`.
- **Lifetimes:** the C++ listener object is leaked intentionally (process
  lifetime) so a late callback can never hit freed memory; the Rust
  `Box<Sender<Event>>` passed as `user` is dropped only after `zc_uninit` returns
  and the callback pointer has been cleared under the adapter mutex.
- **Lifecycle order** mirrors `PSDKTestDlg.cpp`: `InitZMToolSuite` →
  `StartToolSuiteAuth` → `SetToolSuiteProxyListener(this)`; shutdown
  `UninitZMToolSuite` → `SetToolSuiteProxyListener(nullptr)`.
- **Runtime placement (verified):** the complete SDK `bin\` set must sit beside the
  exe. Loading `zToolSuiteIPCProxy.dll` by absolute path from the SDK folder
  initialises without error but **no auth/IPC callback ever arrives**.
  `scripts\build.ps1` syncs `bin\` into `target\<profile>\`; `connect` refuses to
  start otherwise.
- **DLL loading:** delay-loaded; `ensure_sdk_dll_loaded` first tries the exe dir,
  then the build-time SDK `bin\` by absolute path (`LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR`
  so dependencies resolve from the same folder). Distribution: `scripts\package.ps1`
  copies the full `bin\` set beside the exe (official requirement). The SDK is
  process-global; one `Session` per process (`AlreadyActive` otherwise).
- **Secrets:** token is passed UTF-8 → wiped `std::wstring` after
  `StartToolSuiteAuth`; Rust copies are zeroed; `Secret` never formats its value.

## OAuth

Authorization Code + PKCE (S256), `state` check, loopback redirect bound to
127.0.0.1/::1 only on the registered port, system browser via `ShellExecuteW`,
token exchange with `client_id` (public client) — no backend, no secret in the
binary. Tokens (incl. rotated refresh tokens) stored with DPAPI
(`CryptProtectData`, user scope, app entropy). Refresh happens automatically in
`zoom_auth::valid_tokens` when < 2 minutes remain.
