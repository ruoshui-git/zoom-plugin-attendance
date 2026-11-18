# Microsoft Store: free signing, hosting and updates

Why the Store: an unsigned download triggers Windows SmartScreen ("Windows protected your
PC"). A code-signing certificate normally costs money. The Microsoft Store is the free
route: developer registration is free for individuals (since Sept 2025) and companies
(since May 2026), and Microsoft signs MSIX packages it publishes, hosts them, and delivers
updates automatically.

**MSIX, not EXE/MSI.** The Store also accepts classic EXE/MSI installers, but only if you
sign them yourself with a paid certificate, you host the download, and you build your own
updater. With MSIX the Store does all three for free. `scripts\package.ps1 -Msix`
already produces it.

**After Zoom approval.** Zoom's API terms (§6.1) allow distributing the app to third
parties only once it is published on the Zoom App Marketplace, so submit to the Store
after Zoom approves (Store certification testers are third parties too).

Other options, for reference:

| Option | Cost | Notes |
|---|---|---|
| **Microsoft Store (MSIX)** | Free | Recommended. Microsoft re-signs the package after certification |
| Azure Artifact Signing (formerly Trusted Signing) | Monthly fee | Individual identity validation was limited to the US/Canada as of 2026; check eligibility in the Azure portal |
| SignPath Foundation, Certum Open Source | Free / low | Only for open-source projects (this repo is not) |
| OV/EV certificate from a CA | Paid yearly | Standard route for direct downloads |
| Unsigned zip | Free | Fine for a few testers and Zoom's reviewers: *More info* > *Run anyway* |

## Steps

1. **Register** at https://storedeveloper.microsoft.com (done, identity verified; no other
   verification is needed to submit an app).
2. **Reserve the name** (fine to do now): Partner Center > Apps and games > New product > *MSIX or PWA app*
   > "AttendFlow Desktop".
3. **Copy the identity**: Product management > *Product identity*. You need
   `Package/Identity/Name`, `Package/Identity/Publisher` (`CN=...`) and
   `Package/Properties/PublisherDisplayName`.
4. **Build the package** (the production public client ID is the default):
   ```powershell
   .\scripts\package.ps1 -Msix `
       -IdentityName '<Package/Identity/Name>' `
       -Publisher '<Package/Identity/Publisher>' `
       -PublisherDisplayName '<PublisherDisplayName>'
   ```
   Output: `dist\AttendFlowDesktop-<version>-x64.msix` (unsigned is fine for upload).
   Bump `version` in the root `Cargo.toml` for every new upload; the Store rejects a
   version it has seen before.
5. **Test the package locally first** (Developer Mode on):
   ```powershell
   Add-AppxPackage -Register .\dist\msix-layout\AppxManifest.xml
   # start "AttendFlow Desktop" from the Start menu, sign in, join a test meeting
   Get-AppxPackage *AttendFlow* | Remove-AppxPackage
   ```
6. **Create the submission**:
   - *Pricing and availability*: Free, public visibility (Zoom has approved by now).
   - *Properties*: category **Productivity**; privacy policy URL
     `https://www.attendflow.org/privacy#desktop`; website `https://www.attendflow.org/desktop`;
     support contact `support@attendflow.org`.
   - *Age ratings*: answer the questionnaire (no user-generated content shared, no
     purchases, no location; the app collects names of meeting participants and keeps them on the device).
   - *Packages*: upload the `.msix`.
   - *Store listings* (English and Chinese (Simplified)): reuse the Zoom long description
     from `zoom-marketplace.md`; at least one screenshot (1366 x 768 or larger).
   - *Submission options > Restricted capabilities* (`runFullTrust`):
     > AttendFlow Desktop is a Win32 desktop application. It loads Zoom's Plugin SDK
     > (native DLLs) and communicates with the locally installed Zoom Workplace client over
     > IPC to read the participant list of the meeting the user is in. This requires a
     > full-trust desktop process.
   - *Notes for certification*:
     > Requires Zoom Workplace for Windows (7.0.2+) running and signed in, and a Zoom
     > account. Without Zoom the app opens and shows a "Sign in with Zoom" banner;
     > Settings, Configuration (Load example data) and Records are usable offline.
     > Sign-in uses the system browser (OAuth PKCE with a loopback redirect).
7. Certification usually takes a few business days. After it passes, put the Store URL in
   `zoom-tracker/www/src/lib/desktop.ts` (`desktopStoreUrl`) and redeploy the website.

## Data location note

Packaged (MSIX) apps may have their `%LOCALAPPDATA%` writes redirected to a per-package
folder that Windows deletes on uninstall. The in-app paths do not change, and the
documentation already tells users to export what they want to keep before uninstalling.
