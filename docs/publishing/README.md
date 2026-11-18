# Publishing AttendFlow Desktop: step-by-step

How real users get the app:
1. They install AttendFlow Desktop (Microsoft Store).
2. On first launch they click **Sign in with Zoom** and allow access in the browser.
3. They join meetings as usual; attendance is captured automatically.

They never have to find the app on the Zoom Marketplace. But Zoom only lets people
outside your own Zoom account complete step 2 once the Marketplace app is **published**
(the development client ID works only for your account), and Zoom's API terms (§6.1)
allow distributing the app to third parties only after it is published on the Marketplace.
So the order is: **Zoom review first, then the Microsoft Store.**

Companion documents:
- [zoom-marketplace.md](zoom-marketplace.md): every field, justification, security answer and the test plan
- [microsoft-store.md](microsoft-store.md): free signing and distribution via the Store

## Status

| Item | Status |
|---|---|
| In-app sign-in/sign-out (revokes at Zoom), status banner, removal detection | done (`release-prep`) |
| Production public client ID `_up3N0AUTXmUr5u0F74umg` built in by default | done (`scripts\package.ps1`) |
| Launcher, icon, single instance, local-server hardening, zip + MSIX packaging | done |
| Website `/desktop`, privacy `#desktop`, terms, support links; docs install/use/remove | done (zoom-tracker `desktop-pages`, not deployed) |
| Zoom account compliance (credit card / developer form) | done (owner, for AttendFlow) |
| Microsoft Store developer account + identity verification | done (owner) |
| Support hours/SLA | same wording as AttendFlow's support page (accepted in AttendFlow's review) |
| Loopback redirect allowed for public clients | confirmed in Zoom's PKCE docs; no hosted redirect page needed |
| Zoom SDK license | confirmed: see step 5 |
| Public repo + GitHub security scans | pending owner decision (history contains your personal meeting ID and email) |
| SDK connection inside MSIX | **not verified yet** (owner test, step 2) |

## Your steps

### Step 1: production credentials (Marketplace > AttendFlow Desktop > Production)
- [ ] *Use Public Client OAuth*: on. Justification text: `zoom-marketplace.md` section 1.
- [ ] PKCE OAuth Redirect URL and OAuth allow list: `http://127.0.0.1:47811/callback`.
- [ ] Scope `plugin_sdk:read:connection_meta`; Features > Access > Plugin SDK enabled.

### Step 2: build and test the production package yourself
```powershell
.\scripts\package.ps1 -Msix        # production public client ID is the default
```
1. Rename `%APPDATA%\zoom-companion\config.toml` while testing: its dev `client_id`
   **overrides** the built-in ID (`dist\AttendFlowDesktop\zoom-companion.exe config show`
   prints the ID in effect). Rename it back afterwards for development.
2. Make sure Zoom Workplace is running and signed in with your account.
3. Zip build: run `dist\AttendFlowDesktop\AttendFlowDesktop.exe`, Sign in with Zoom, join a
   test meeting (a guest from the web client), check Live and Records, then Settings >
   Sign out of Zoom.
4. MSIX build: `Add-AppxPackage -Register .\dist\msix-layout\AppxManifest.xml`, start
   "AttendFlow Desktop" from the Start menu, repeat, then
   `Get-AppxPackage *AttendFlow* | Remove-AppxPackage`. This is the check I could not finish
   (Zoom Workplace returned `IPC_STATUS_CONNECT_TIMEOUT`, its answer when it is signed out
   or signed in as a different user).

### Step 3: website
- [ ] Review/merge zoom-tracker branch `desktop-pages`.
- [ ] Copy `dist\AttendFlowDesktop-<ver>-x64.zip` to `zoom-tracker\www\public\downloads\`
      (git-ignored). Keep `desktopZipUrl` empty in `www/src/lib/desktop.ts` so the public page
      does not offer it before approval; give reviewers the direct URL
      `https://www.attendflow.org/downloads/AttendFlowDesktop-<ver>-x64.zip` in the test plan.
- [ ] Deploy: `firebase deploy --only hosting:www,hosting:docs` (from `zoom-tracker`).

### Step 4: submit to Zoom
- [ ] Listing, URLs, technical design, scope justification and test plan from `zoom-marketplace.md`.
- [ ] Security evidence: GitHub code scanning / Dependabot results once the repo is public
      (as you did for AttendFlow), plus `cargo audit` and `pnpm audit` output.
- [ ] Optional: 3-5 minute demo video of the test plan (unlisted link).

### Step 5: Zoom SDK license (resolved)
Zoom's API License and Terms of Use cover the client SDKs ("Zoom APIs" includes "Zoom's
Client software development kits"). Findings:
- Shipping the SDK DLLs inside the app is allowed: the prohibition on distributing "the
  Zoom APIs" has the exception "except as integrated within your Application".
- No license file, notice or attribution is required for the SDK binaries. Nothing to add.
- §6.1: distribution to third parties only after the app is **published on the Zoom App
  Marketplace** (or with Zoom's written approval). Hence Store and public download after approval.
- §7.5/§13.2: do not suggest affiliation with Zoom or use Zoom logos beyond the brand guidelines.
- The source repo never contains SDK files (`vendor/` is git-ignored), so making it public
  does not distribute the SDK.

### Step 6: Microsoft Store (after Zoom approval)
Upload the **MSIX** (not an EXE/MSI installer): see [microsoft-store.md](microsoft-store.md).
Then set `desktopStoreUrl` in `www/src/lib/desktop.ts` and redeploy.
