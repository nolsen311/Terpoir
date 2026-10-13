<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Releasing

How a release goes from a tag to signed APKs on GitHub and a signed bundle
for Google Play. CI builds; only the maintainer signs, offline, with the
YubiKey (`docs/design-decisions.md`, "Signing keys"). Signing material
never reaches CI or the repository.

## One-time setup (maintainer's machine)

1. **Tools:**
   - `yubico-piv-tool`, which provides `libykcs11`, the YubiKey's PKCS#11
     module, and `ykman`;
   - the Android build-tools (`apksigner`, `zipalign`) on `PATH`;
   - a JDK 17 or newer (`jarsigner`, `keytool`);
   - `gh`, logged in.
2. **The app signing key's slot.** `ykman piv info` lists the slots that
   hold a key and certificate. The script uses slot `9a` unless
   `TERPOIR_APP_SLOT` says otherwise.
   - **PIN policy:** Java's PKCS#11 provider, which `apksigner` and
     `jarsigner` use, can't handle a key whose PIN policy is "always" (a
     PIN for every signature). That is slot `9c`'s default. If signing
     fails with `CKR_USER_NOT_LOGGED_IN`, the key needs PIN policy "once":
     re-import it from the offline backup with `--pin-policy ONCE`
     (keeping `--touch-policy ALWAYS`), into both YubiKeys.
3. **Pin the certificate.** Its SHA-256 fingerprint goes into
   `android/release/app-certificate.sha256` in a PR. From then on, the
   script refuses an APK signed by any other key:
   ```bash
   ykman piv certificates export 9a - | openssl x509 -noout -fingerprint -sha256
   ```
   The same certificate's SHA-1 (`-fingerprint -sha1`) goes into
   Terpoir-infra's `maps_release_sha1s`, for the release Google Maps key.
   The SHA-256 is also published in the README and on terpoir.app (Phase 4).
4. **The Play upload key** (only once the Play developer account exists).
   It signs the bundle uploaded to Play, and Google re-signs the app with
   the app signing key it holds (Play App Signing). Lower stakes than the
   app key: Google can reset a lost upload key. So generate it on the
   primary YubiKey itself, in a retired slot such as `82`:
   ```bash
   ykman piv keys generate --algorithm ECCP256 --pin-policy ONCE --touch-policy ALWAYS 82 upload.pub.pem
   ykman piv certificates generate --subject "CN=Terpoir upload key" --valid-days 10000 82 upload.pub.pem
   ykman piv certificates export 82 upload.pem   # for the Play Console
   ```
   Then set `TERPOIR_UPLOAD_SLOT=82` when signing. (The first release used
   slot `9d` for the upload key instead; any slot works, as long as
   `TERPOIR_UPLOAD_SLOT` names it.)

## Each release

**Shortcut:** once the version PR is merged, `android/scripts/release.sh
v0.1.0` does steps 2 to 5 below in one run. It asks before it tags and
pushes, signs the tag, waits for CI, runs `sign-release.sh --upload` (PIN
once, a touch per signature), and asks before publishing. It is safe to
re-run after an interruption. Set `TERPOIR_UPLOAD_SLOT=82` (and
`TERPOIR_SSH_KEY`) in the environment as for `sign-release.sh`. It writes
signed builds under `~/terpoir-releases` (`TERPOIR_WORK_DIR`). Nothing is
signed in CI: it only chains the steps by hand.

The steps, for doing them separately:

1. **Version:** a PR raises `versionCode` and sets `versionName` in
   `android/app/build.gradle.kts`. Merge it.
2. **Tag** that commit on `main`, signed with the YubiKey's SSH key, and
   push the tag:
   ```bash
   git tag -s v0.1.0 -m "Terpoir 0.1.0"
   git push origin v0.1.0
   ```
3. **CI** (`.github/workflows/release.yml`):
   - checks the tag is on `main` and matches `versionName`;
   - builds the unsigned foss and Play APKs and the Play bundle, and runs
     every check;
   - creates a draft release holding them and `UNSIGNED-SHA256SUMS`.

   The build runs in the `release` environment, which holds the release
   Google Maps key once Terpoir-infra provides it. Without the key, the
   Play build doesn't offer Google Maps.
4. **Sign** with the YubiKey plugged in (one PIN entry, a touch per
   signature):
   ```bash
   TERPOIR_UPLOAD_SLOT=9d android/scripts/sign-release.sh v0.1.0 --upload
   ```
   The script:
   - downloads the unsigned builds and checks them against CI's hashes;
   - signs both APKs with the app key and the bundle with the upload key;
   - verifies every signature and the pinned certificate;
   - writes `SHA256SUMS`, and signs it if `TERPOIR_SSH_KEY` names an
     `ed25519-sk` key;
   - with `--upload`, replaces the unsigned files on the draft with the
     signed APKs and `SHA256SUMS`.

   The signed bundle stays local.
5. **Publish:** check the draft, then publish it:
   ```bash
   gh release edit v0.1.0 --draft=false
   ```
   Upload the bundle in the Play Console.

## Notes

- **Signature schemes:** minSdk is 30, so `apksigner` signs with APK
  Signature Scheme v3 only. v1/v2 aren't needed, and v4 (`.idsig` files,
  for incremental installs) is turned off.
- **Signature only:** the signed APK is CI's unsigned APK plus the
  signature, so F-Droid can check its own build against it.
- **Tested:** the script was run end to end against SoftHSM, a software
  PKCS#11 token holding keys under the YubiKey's slot labels: signing,
  verification, and refusing a certificate that doesn't match the pin.
  It has not yet run against a YubiKey.
