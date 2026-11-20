# Zoom Marketplace submission: AttendFlow Desktop

Copy-ready text for every part of the Marketplace build flow and review. The
Marketplace app is the **General app** created for this project (not the AttendFlow
Zoom App). Fields marked **(you)** need a value only the owner has.

Sources checked 2026-11-18: [PKCE / public clients](https://developers.zoom.us/docs/integrations/pkce/),
[common rejection reasons](https://developers.zoom.us/docs/distribute/app-submission/common-rejection-issues/),
[SDK review requirements](https://developers.zoom.us/docs/distribute/sdk-feature-review-requirements/),
[app review process](https://developers.zoom.us/docs/distribute/app-review-process/),
[private and beta apps](https://developers.zoom.us/docs/distribute/sharing-private-and-beta-apps/).

---

## 1. Credentials and OAuth (Production tab)

| Setting | Value |
|---|---|
| App type | General app, **user-managed** |
| Use Public Client OAuth | **On**. Production public client ID `_up3N0AUTXmUr5u0F74umg` is the default in `scripts\package.ps1` |
| OAuth redirect URL (required field, not used by the app) | `https://www.attendflow.org/desktop` |
| PKCE OAuth Redirect URL | `http://127.0.0.1:47811/callback` |
| OAuth allow list | `http://127.0.0.1:47811/callback` and `https://www.attendflow.org/desktop` |
| Scopes | `plugin_sdk:read:connection_meta` (only this one) |
| Features > Access | Plugin SDK enabled (Windows) |
| Deauthorization | Zoom's standard removal (Option 1). No endpoint: the app has no server. See section 6 |

Loopback is officially supported for PKCE public clients: "Use a numeric loopback
literal (`127.0.0.1` or `[::1]`); do not use `localhost`" and "Zoom ignores only the
port", so each sign-in binds a fresh ephemeral port and `47811` in the registered URL
is only a placeholder. This applies to the **public client ID** only: with the
confidential Client ID, Zoom checks the port and shows "Invalid redirect ... (4,700)".

The plain "OAuth redirect URL" is required on the Production tab but rejects loopback
addresses. The app never uses it; it is where Zoom sends someone who clicks "Add" on the
Marketplace listing, so the download page is the right target.

### Why a public client ("Use Public Client OAuth" justification)

Short version for the form field:

> AttendFlow Desktop is a native Windows desktop app with no backend server, so it cannot
> keep a client secret confidential: anything shipped in the executable can be extracted.
> Following RFC 8252 (OAuth for native apps), it uses the public client ID with
> Authorization Code + PKCE (S256), the system browser, and a loopback redirect
> (http://127.0.0.1:<ephemeral port>/callback) that listens only during sign-in and checks
> `state`. Tokens are stored encrypted with Windows DPAPI for the current user and sent only
> to zoom.us. The only scope is plugin_sdk:read:connection_meta, used solely to authenticate
> the Zoom Plugin SDK connection to Zoom Workplace on the same computer.

Long version (if the review asks for detail):

> AttendFlow Desktop is a native Windows desktop application with no backend server.
> A client secret shipped inside a desktop executable cannot be kept confidential, so the
> app uses the public client ID with Authorization Code + PKCE (S256), as recommended
> for native apps by RFC 8252. The browser redirects to a loopback address
> (`http://127.0.0.1:<ephemeral port>/callback`) that the app listens on only for the
> duration of the sign-in; the `state` parameter is verified. Tokens are stored on the
> user's computer encrypted with Windows DPAPI (current user scope) and are only sent
> to zoom.us. The token is used for a single purpose: authenticating the Zoom Plugin SDK
> connection to the Zoom Workplace client on the same computer.

### Scope justification

> **plugin_sdk:read:connection_meta**: required by the Zoom Plugin SDK. The app passes
> the user's access token to `StartToolSuiteAuth`, which lets Zoom Workplace verify that
> the user signed in to AttendFlow Desktop is the same user signed in to the Zoom client
> before it opens the local IPC connection. The app reads the in-meeting participant list
> and join/leave events through that connection to record attendance. It calls no Zoom
> REST APIs and requests no other scopes.
>
> **How to test:** sign in with Zoom from the app, then join a meeting in Zoom Workplace
> with the same account; the Live page lists participants within a few seconds.

---

## 2. Listing

| Field | Value |
|---|---|
| App name | AttendFlow Desktop |
| Short description (max 150) | Record Zoom meeting attendance on your own Windows PC, even as a participant. Roster matching, CSV/JSON export, data stays local. *(129 chars)* |
| Category | Productivity (secondary: Education, if offered) |
| Developer / company name | **(you)** |
| Contact email | support@attendflow.org |
| Landing / "Add" page | https://www.attendflow.org/desktop *(if the build flow offers "From your site", choose it: the app is added by installing it, not from the Marketplace page)* |
| Documentation URL | https://docs.attendflow.org/desktop/install-use-remove/ |
| Privacy policy URL | https://www.attendflow.org/privacy#desktop |
| Terms of use URL | https://www.attendflow.org/terms |
| Support URL | https://www.attendflow.org/support |
| Logo | `packaging/assets/icon-source.png` (resize to the size the form asks for) |

All URLs are on attendflow.org, which you own (Zoom rejects documents hosted on Google
Drive, Dropbox, etc.).

### Long description

> AttendFlow Desktop records attendance for the Zoom meetings you attend, right on your
> Windows computer, even when you are not the host or co-host.
>
> It runs next to the Zoom app (Zoom Workplace). Sign in with Zoom once; from then on,
> whenever you are in a meeting, AttendFlow Desktop records who joined, when they left
> and when they came back, using the same participant list the Zoom app already shows you.
>
> **Features**
> - Live attendance: present, left and returned participants with join and leave times.
> - Works as a participant: no host or co-host role needed.
> - Roster mode: import a roster (CSV) and see who is missing; match Zoom display names to
>   roster names, with aliases remembered for next time.
> - Meeting profiles: late and return thresholds per class or team.
> - Records: one record per meeting, with entry/exit history, raise-hand captures and name changes.
> - Export to CSV or JSON. English and Simplified Chinese.
>
> **Privacy by design**
> AttendFlow Desktop has no server and no AttendFlow account. Attendance data, rosters and
> settings are stored only on your computer. The app requests a single Zoom permission,
> which lets it connect to the Zoom app on the same computer; it cannot read your meetings,
> recordings or contacts through the Zoom website.
>
> **Requirements:** Windows 10 or 11 (64-bit), Zoom Workplace for Windows 7.0.2 or later,
> signed in with the same Zoom account you use for AttendFlow Desktop.
>
> Please make sure meeting participants are informed that attendance is recorded, as
> required by your organization and local law.

### Gallery (2-3 images, 1200 x 780 px)

1. Live page during a meeting (roster mode, a few participants present, one left).
2. Records list + record detail (attendance tab).
3. Sign-in banner (first run), or Name Matching.

Capture them with the fake meeting: `zoom-companion serve --fake --port 47820`, open
`http://127.0.0.1:47820/`, click Settings > Load example data (or run
`apps/desktop-ui/scripts/screenshots.mjs`), then crop/resize to 1200 x 780.

---

## 3. Technical design and security (build flow > Technical Design / security questions)

**Architecture.** A single Windows process (`zoom-companion.exe`, Rust) plus a launcher.
It loads Zoom's official Plugin SDK (`zToolSuiteIPCProxy.dll`, Authenticode-signed by
Zoom) and connects over local IPC to the Zoom Workplace client on the same computer. A
local HTTP/WebSocket server bound to `127.0.0.1` serves the bundled UI to an embedded
WebView2 window. There is no backend, database server or cloud component.

**Data flow.** Zoom Workplace -> Plugin SDK IPC -> attendance engine -> local SQLite
database (`%LOCALAPPDATA%\zoom-companion\data.db`) -> UI on 127.0.0.1. The only outbound
network traffic is to `https://zoom.us/oauth/{authorize,token,revoke}`.

**Data collected.** Meeting number and topic; participants' display names, Zoom user
IDs, host/co-host/waiting-room flags, raised hands, join/leave timestamps; user-entered
rosters, aliases and settings; local diagnostic logs. None of it is transmitted to the
developer or third parties.

**Use of Zoom data.** Only to compute and display attendance for the signed-in user's
meetings, on the user's computer. No analytics, no advertising, no AI training, no sale.

**Token handling.** Authorization Code + PKCE (S256) with `state` validation; loopback
redirect on 127.0.0.1 with an ephemeral port (listening only during sign-in); tokens encrypted at rest with Windows
DPAPI (`CryptProtectData`, current-user scope, app-specific entropy); in-memory copies
zeroed after use; never logged. Refresh tokens are rotated and re-persisted. Sign-out
calls `POST /oauth/revoke` and deletes the token file.

**Encryption in transit.** All calls to zoom.us use HTTPS (rustls, TLS 1.2+). Local IPC
is handled by Zoom's SDK. The local UI server listens on the loopback interface only.

**Local server hardening.** Binds 127.0.0.1 only; rejects foreign `Origin` headers
(CSRF) and foreign `Host` headers (DNS rebinding); state-changing account endpoints
require the app's own Origin; one instance per user (second instance cannot bind).

**Encryption at rest.** Tokens: DPAPI. Attendance database: protected by the Windows user
profile's file permissions (and BitLocker if the user enables it); not separately encrypted.

**Retention and deletion.** Data stays until the user deletes it (Settings > Clear local
records; uninstall + delete folder). Sign out revokes the grant. Documented at
https://docs.attendflow.org/desktop/install-use-remove/.

**Third parties / subprocessors.** None for the desktop app. (Microsoft Store distributes
the installer, if used.)

**Secure development.** Rust (memory-safe) with a thin C++ adapter around the SDK; unit
and integration tests (engine parity suite, API tests, UI tests); dependency lockfiles;
no secrets in the repository. Before submission, run and keep the reports from:
`cargo audit` (or `cargo deny check advisories`), `cargo clippy --workspace`,
`pnpm audit` in `apps/desktop-ui`.

**Vulnerability reporting.** security reports to support@attendflow.org.

### Technical Design form fields

**Technology stack** (paste):

> AttendFlow Desktop is a native Windows 10/11 (x64) desktop application with no backend
> server.
> - Languages: Rust (application), C (thin adapter around the Zoom SDK), TypeScript and
>   Svelte (user interface).
> - Zoom: Zoom Plugin SDK for Windows (zToolSuiteIPCProxy.dll, unmodified, from Zoom).
>   It authenticates with StartToolSuiteAuth and reads the in-meeting participant list and
>   join/leave events from Zoom Workplace over local IPC.
> - Zoom OAuth 2.0: /oauth/authorize, /oauth/token and /oauth/revoke, using Authorization
>   Code + PKCE (S256) as a public client. Only scope: plugin_sdk:read:connection_meta.
>   No Zoom REST APIs, webhooks or Zoom Apps SDK.
> - Windows APIs: DPAPI (CryptProtectData) for token encryption, Microsoft Edge WebView2
>   for the app window, ShellExecute to open the system browser for sign-in.
> - Rust libraries: tokio and axum (local HTTP/WebSocket server bound to 127.0.0.1),
>   ureq with rustls (HTTPS/TLS 1.2+ to zoom.us), rusqlite with bundled SQLite (local
>   storage), wry/tao (WebView2 window), serde, sha2/base64/getrandom (PKCE), url, chrono,
>   clap, log, windows-sys.
> - UI libraries: Svelte 5, Luxon; built with Vite.
> - Storage: a local SQLite file and a DPAPI-encrypted token file in the user's profile.
>   No cloud storage or databases.
> - Build and distribution: Cargo, pnpm, MSVC; MSIX package through the Microsoft Store.
> - Security tooling: GitHub CodeQL (Rust, C/C++, JavaScript/TypeScript), Dependabot,
>   secret scanning with push protection.

**Architecture diagram:** upload `docs/publishing/review/architecture-diagram.png`
(source: `architecture-diagram.html`; re-render with Edge headless if it changes).

**Application development:**

| Question | Answer | Upload |
|---|---|---|
| Secure software development process (SSDLC)? | **Yes** | `review/pdf/ssdlc.pdf` |
| SAST and/or DAST? | **Yes** (SAST: CodeQL; the report says plainly that there is no hosted DAST because the app exposes no network service) | `review/pdf/sast-dast-evidence.pdf` |
| Periodic third-party penetration testing? | **No** | none |

**Additional documents:**

| Item | File |
|---|---|
| Vulnerability management policy | `review/pdf/vulnerability-management-policy.pdf` |
| Security policy | `review/pdf/security-policy.pdf` |
| Privacy policy | print https://www.attendflow.org/privacy to PDF after the `desktop-pages` branch is deployed (it has the `#desktop` section) |
| Data retention/protection policy | `review/pdf/data-retention-and-protection-policy.pdf` |
| Incident management and response policy | `review/pdf/incident-response-policy.pdf` |
| Infrastructure/dependency management policy | `review/pdf/infrastructure-and-dependency-management-policy.pdf` |
| SOC2, ISO 27001 | leave empty (none) |

Re-generate the PDFs after editing the Markdown sources:
`.\docs\publishing\review\export-pdfs.ps1` (needs pandoc and Edge). Update the numbers
in `sast-dast-evidence.md` from the GitHub Security tab first.

---

## 4. Test plan (paste into "Test plan"; required on every submission)

> **App type:** native Windows desktop application using the Zoom Plugin SDK.
> **No AttendFlow account exists**; reviewers use their own Zoom account. No test
> credentials are needed.
>
> **Prerequisites**
> - Windows 10/11 x64 with Zoom Workplace for Windows 7.0.2 or later, signed in with the
>   reviewer's Zoom account.
> - A second participant (another device, or the Zoom web client in a browser) to join the test meeting.
>
> **Install**
> 1. Download: https://www.attendflow.org/downloads/AttendFlowDesktop-0.1.0-x64.zip
>    The zip is not yet code-signed; if Windows SmartScreen appears, click *More info* > *Run anyway*.
> 2. Extract the zip and run `AttendFlowDesktop.exe` (no installation or admin rights needed).
>
> **TC1: Authorization (production client ID)**
> 1. The app opens with a "Sign in with Zoom" banner. Click **Sign in with Zoom**.
> 2. The browser opens zoom.us/oauth/authorize for AttendFlow Desktop. Sign in with the same
>    account as Zoom Workplace and click **Allow**.
> 3. Expected: the browser shows "Zoom sign-in complete. You can close this tab and return to AttendFlow Desktop."; the app's status
>    (bottom left) changes to **Connected to Zoom** within a few seconds.
>
> **TC2: Attendance capture**
> 1. Start a meeting in Zoom Workplace. Join it from the second device/web client as "Test Guest".
> 2. Expected: the **Live** page shows the meeting and both participants with join times.
> 3. Have "Test Guest" leave. Expected: the guest is shown as left with a leave time.
> 4. End the meeting. Open **Records**: the meeting appears; open it to see the attendance
>    tab; click **Export** to save a CSV.
>
> **TC3: Account mismatch (optional)**
> Sign Zoom Workplace in with a different account while the app is signed in. Expected: the
> app shows "Different Zoom accounts" / "The Zoom app did not respond" with guidance.
>
> **TC4: Deauthorization and removal**
> 1. In the app: **Settings** > **Sign out of Zoom** > confirm. Expected: "Signed out and
>    access removed"; status **Not signed in**; the app no longer appears under Zoom App
>    Marketplace > Manage > Added Apps.
> 2. Alternatively remove the app from Marketplace > Manage > Added Apps while the app is
>    running. Expected: on its next connection attempt the app reports that the sign-in
>    expired and asks to sign in again.
> 3. Uninstall: delete the extracted folder (zip) or uninstall from Windows Settings (Store).
>    Local data handling is described at https://docs.attendflow.org/desktop/install-use-remove/.

Optional but recommended: a 3-5 minute demo video following TC1-TC4 (Zoom calls it
"highly helpful"). Upload it unlisted (e.g. YouTube) and link it in the test plan.

---

## 5. Requirements Zoom checks that are already handled in the app

| Requirement | Where |
|---|---|
| Production client ID in the build | `package.ps1 -ClientId` (baked in; `config show` prints it) |
| Least privilege scopes | one scope |
| User documentation: add, use, remove, data after removal | docs.attendflow.org/desktop/install-use-remove |
| Support URL with response expectations | attendflow.org/support (same wording as AttendFlow's accepted listing) |
| Deauthorization | Option 1 (Zoom standard removal) + in-app revoke |
| Policies on own domain | attendflow.org |

## 6. Deauthorization note (paste if asked)

> AttendFlow Desktop has no server and stores no Zoom data off the user's computer, so
> there is no server-side data to delete on deauthorization; we use Zoom's standard
> removal process. In the app, Settings > Sign out of Zoom calls `POST /oauth/revoke` and
> deletes the locally stored tokens. If the user removes the app from the Marketplace
> instead, the app detects the revoked grant on its next connection attempt (refresh is
> rejected), deletes the local tokens and asks the user to sign in again. Local attendance
> records remain under the user's control and can be deleted in the app or by removing the
> app's data folder, as documented.
