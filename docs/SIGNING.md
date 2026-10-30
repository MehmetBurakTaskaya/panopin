# Signing

Panopin uses two kinds of signatures. Update signatures are in place from the first release;
code signing (Authenticode) is not, because certificates cost money and the project uses only free
tools.

## What users see today

The installer and `Panopin.exe` are not code-signed, so Windows SmartScreen may stop the first
start with "Windows protected your PC". Choose **More info → Run anyway**. The warning comes from
missing reputation, not from anything found in the file, and each new version starts without
reputation again.

On macOS the equivalent is **System Settings → Privacy & Security → Open Anyway** (see "macOS"
below).

Every release lists the SHA-256 of each file in `SHA256SUMS.txt`. To compare a download in
PowerShell:

```powershell
Get-FileHash .\Panopin-Setup-x64.exe -Algorithm SHA256
```

## Update signatures

The updater installs only packages signed with the project's updater key (D-028, D-065).

- The key pair was made with `tauri signer generate`. The public key is in
  `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).
- The private key exists only in the repository's GitHub Secrets, as
  `TAURI_SIGNING_PRIVATE_KEY` (and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` when it has a password).
  It is never committed. The owner keeps an offline backup, for example in a password manager.
- `release.yml` builds the installer, signs it with that key and publishes the signature in
  `latest.json`. It signs the portable ZIP with the same key for its version
  (`tauri signer sign --app-version`), so the portable copy can update itself (D-076).
- Every update signature carries the version it was made for, and the app refuses one whose
  version differs from what `latest.json` announces (`requireSignedVersion`).
- If the private key is lost, installed copies cannot accept updates signed with a new key: users
  have to install the next version by hand once.
- To replace the key: make a new pair, ship one release signed with the old key that carries the
  new public key, then switch the secret to the new private key.

## Free code signing through SignPath Foundation

SignPath Foundation signs open-source projects for free, with certificates it issues itself. Its
conditions (signpath.org/terms, read on 2026-10-30) and where Panopin stands:

| Condition | Panopin |
| --- | --- |
| Every component under an OSI-approved licence, no proprietary parts | MIT; dependencies limited to the allow-list in `src-tauri/deny.toml` |
| No malware, no potentially unwanted programs, no hacking tools | Yes |
| Actively maintained, and already released in the form to be signed | After v1.0.0 is published |
| Functionality described on the download page | README and `site/` |
| Only the team's own binaries, built verifiably from source | `Panopin.exe` and the installer are built by `release.yml` on GitHub Actions; the WebView2 bootstrapper is downloaded by the installer, not packed into it |
| Data sent to other systems is described in a privacy policy, shown during installation, and can be turned off | Link previews are off until turned on and can be turned off in Settings. Update checks are always on (D-076): they fetch `latest.json` and send nothing about the user. **To do:** show the privacy note during installation (for example on an installer page), and ask SignPath whether an update check that cannot be turned off is acceptable |
| No system changes without warning; an uninstaller | Start with Windows is opt-in; the installer adds an uninstaller |
| Multi-factor authentication for every team member on GitHub and SignPath | **To do:** confirm MFA on both accounts |
| Code signing roles: authors, reviewers, approvers | **To do:** name them (today all three are Mehmet Burak Taşkaya) |
| A "Code signing policy" section on the home page and on download or release pages | **To do:** add it to the README, `site/` and the release notes, with the statement "Free code signing provided by SignPath.io, certificate by SignPath Foundation", the roles and a link to the privacy section |
| Artifact configuration: product name equals the project name, the same product version in each build, no upstream binaries signed | **To do** when setting up the SignPath project |
| Manual approval of every signing request | Accepted |

### Steps

1. Publish v1.0.0 and finish the "To do" rows above.
2. Apply at <https://signpath.org/apply>.
3. When accepted, add SignPath's GitHub integration to `release.yml`. The order matters: sign
   `Panopin.exe` before it is bundled (Tauri's `bundle.windows.signCommand` runs a custom signing
   command for that), sign the installer, and only then create the update signature, because it
   must cover the final installer bytes.
4. Keep the SmartScreen note in the README until a signed release has been out for a while.

## macOS

The Mac app is signed ad hoc (`signingIdentity` `-` in `src-tauri/tauri.macos.conf.json`), which
Apple silicon needs, but it has no Developer ID and is not notarised. macOS 15 and later therefore
block its first opening with "Apple could not verify Panopin". The README gives the way through:
close the warning (**Done** on macOS 15 and later, **Cancel** before), then **System Settings →
Privacy & Security → Open Anyway** and the password. A
Developer ID certificate and notarisation need the Apple Developer Program (99 USD a year); the
owner decided on 2026-10-30 not to join for now (D-075). If that changes:

1. Join, create a "Developer ID Application" certificate and export it as a `.p12` file; create an
   app-specific password for the Apple account and note the Team ID.
2. Add the secrets `APPLE_CERTIFICATE` (the `.p12` in base64), `APPLE_CERTIFICATE_PASSWORD`,
   `APPLE_ID`, `APPLE_PASSWORD` (the app-specific password), `APPLE_TEAM_ID` and
   `KEYCHAIN_PASSWORD`.
3. In the macOS job of `release.yml`, import the certificate into a temporary keychain and pass the
   variables to `tauri build`, which then signs and notarises (its log says when they are missing).
   The first notarisation can take hours.
