<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Releasing

How a release goes from a tag to a signed foss APK on GitHub and a signed
bundle on Google Play. GitHub offers only the foss APK (maintainer, 2026-10-13):
the Play build's text recognition comes from Google Play, so a Play APK
installed from GitHub can't have it. CI builds; only the maintainer signs, offline, with the
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

## Checking the YubiKeys

`android/scripts/check-yubikeys.sh` checks the release YubiKeys without a
PIN: it uses no PIN attempt and changes nothing on them. Run it with one
key plugged in, then the other (or both at once). For each key it shows:

- the PIV PIN and PUK tries left, and the FIDO2 PIN tries left. The PIV
  PIN signs the APK and the bundle; the FIDO2 PIN signs the tag and
  `SHA256SUMS`. Below the maximum means a wrong PIN was entered since the
  last correct one. The PIV PIN locks after 3 wrong tries in a row and
  only the PUK unlocks it; if the PUK locks too, the only way back is a
  reset that erases the key's PIV keys. So after a failure, check here
  before trying again;
- each signing slot's key (algorithm, PIN and touch policy, imported or
  generated on the key) and certificate fingerprints;
- whether the app signing slot holds the pinned certificate, and whether
  the upload key slot holds a key at all. An upload key generated on a
  YubiKey exists only on that key: the other can't sign Play bundles.

It saves each key's slot details under `~/terpoir-releases/yubikey-check`
and compares every saved key with the others, so the two keys can be
checked one after the other.

## Google Play (one-time setup)

`android/scripts/play-upload.sh` sends the signed bundle to Google Play
through the Google Play Developer API. It runs on the maintainer's machine,
because the bundle is signed there; `release.sh` runs it as its last step.
There is no key file: the maintainer's own `gcloud` login asks Google's IAM
Credentials API for a 10-minute token of a Play publisher service account,
scoped to the Play Developer API.

1. **The app in the Play Console.** The API can't create an app or make its
   first release. Create the app, then upload the first signed bundle by
   hand (`~/terpoir-releases/terpoir-release-<version>/signed/`). That
   enrolls the app in Play App Signing and registers the upload key.
2. **The service account** (Terpoir-infra, `play.tf`): the Play
   publisher service account, and the Token Creator role on it for the
   maintainer's Google account (the `PLAY_RELEASE_MEMBERS` repository
   variable there). Enable the Google Play Android Developer API in the same
   project (Terpoir-infra BOOTSTRAP B7).
3. **Access in the Play Console:** Users and permissions > Invite new
   users, with the service account's email. Give it access to Terpoir only,
   with "Release apps to testing tracks" (add "Release to production" only
   if production releases should be automated too).
4. **On the maintainer's machine:**
   ```bash
   export TERPOIR_PLAY_SERVICE_ACCOUNT=terpoir-play-publisher@<project>.iam.gserviceaccount.com
   ```
   `gcloud auth login` as a Google account holding the Token Creator role
   on the service account (`gcloud auth list` shows which one is active).

**If Play answers 403 "The caller does not have permission"** while the
token step succeeds and the Play Console shows the service account with the
right permissions: remove the service account in Users and permissions and
invite it again with the same permissions, then wait 10–15 minutes. On
2026-10-22 a week-old invite with correct-looking grants was refused this
way until it was re-invited. To test access without uploading anything,
open and discard an edit:

```bash
TOKEN=$(curl -sS -X POST \
  -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  -H "Content-Type: application/json" \
  -d '{"scope":["https://www.googleapis.com/auth/androidpublisher"],"lifetime":"600s"}' \
  "https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/$TERPOIR_PLAY_SERVICE_ACCOUNT:generateAccessToken" \
  | jq -r .accessToken)
EDIT=$(curl -sS -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" -d '{}' \
  "https://androidpublisher.googleapis.com/androidpublisher/v3/applications/app.terpoir/edits" \
  | tee /dev/stderr | jq -r '.id // empty')
[ -n "$EDIT" ] && curl -sS -X DELETE -H "Authorization: Bearer $TOKEN" \
  "https://androidpublisher.googleapis.com/androidpublisher/v3/applications/app.terpoir/edits/$EDIT" \
  && echo "access works; edit $EDIT discarded"
unset TOKEN
```

**Track and status:** by default the bundle goes to the **internal** track
as a **draft** release, which waits in the Play Console for "Start
rollout". A draft is also the only kind an app that hasn't been published
yet accepts. Later, `TERPOIR_PLAY_TRACK` (`alpha`, `beta`, `production`)
and `TERPOIR_PLAY_STATUS=completed` change both. The upload is one API
edit, committed only once every step succeeded, so a failure changes
nothing on Play. The R8 mapping goes with it, so Play's crash reports show
real names.

## Each release

**Shortcut:** once the version PR is merged, `android/scripts/release.sh
v0.1.0` does steps 2 to 6 below in one run. It asks before it tags and
pushes, signs the tag, waits for CI, runs `sign-release.sh --upload` (PIN
once, a touch per signature), asks before publishing on GitHub, and asks
before sending the bundle to Google Play. It is safe to
re-run after an interruption. Set `TERPOIR_UPLOAD_SLOT=82` (and
`TERPOIR_SSH_KEY`) in the environment as for `sign-release.sh`. It writes
signed builds under `~/terpoir-releases` (`TERPOIR_WORK_DIR`). Nothing is
signed in CI: it only chains the steps by hand.

On the maintainer's machine, run it with the SSH agent switched off:

```bash
SSH_AUTH_SOCK= TERPOIR_UPLOAD_SLOT=9d android/scripts/release.sh v0.2.0
```

The tag and `SHA256SUMS` are signed with the YubiKey's `ed25519-sk` SSH
key. With `SSH_AUTH_SOCK` set, `git tag -s` and `ssh-keygen -Y sign` go
through the desktop's SSH agent, which can't use a security-key key.
Empty, they use the YubiKey directly (touch it when it blinks). The
fetches use HTTPS, so nothing else needs the agent.

The steps, for doing them separately:

1. **Version:** a PR raises `versionCode` and sets `versionName` in
   `android/app/build.gradle.kts`, and adds the version's release notes and
   its **Headline:** and **Summary:** lines to `docs/play-listing.md` (the
   release image uses the headline, the Mastodon post the summary). Merge it.
2. **Tag** that commit on `main`, signed with the YubiKey's SSH key, and
   push the tag:
   ```bash
   git tag -s v0.1.0 -m "Terpoir 0.1.0"
   git push origin v0.1.0
   ```
3. **CI** (`.github/workflows/release.yml`):
   - checks the tag is on `main` and matches `versionName`, and that
     `docs/play-listing.md` has the version's release notes, headline and
     summary, and that its Mastodon post fits;
   - builds the unsigned foss APK and Play bundle, and runs every check;
   - creates a draft release holding them and `UNSIGNED-SHA256SUMS`, its
     page opening with the release notes under "What's new".

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
   - signs the foss APK with the app key and the bundle with the upload key;
   - verifies every signature and the pinned certificate;
   - writes `SHA256SUMS`, and signs it if `TERPOIR_SSH_KEY` names an
     `ed25519-sk` key;
   - with `--upload`, replaces the unsigned files on the draft with the
     signed APK and `SHA256SUMS`.

   The signed bundle stays local.
5. **Publish:** check the draft, then publish it (this also posts it to
   Mastodon, `announce.yml`):
   ```bash
   gh release edit v0.1.0 --draft=false
   ```
6. **Google Play** (see the one-time setup above):
   ```bash
   android/scripts/play-upload.sh v0.1.0
   ```
   Then start the rollout in the Play Console, while releases are drafts.
7. **Compatibility fixtures:** generate a database, an export and sync
   blobs with the new tag's own code (in a separate checkout with its own
   `CARGO_TARGET_DIR`) and check them in under
   `core/crates/terpoir-ffi/tests/fixtures/<version>/` in a PR, as that
   directory's `README.md` describes. `cargo test` then checks that every
   later version still opens them. Not done by `release.sh`.

## Notes

- **Crash stack traces:** release builds are shrunk and obfuscated by R8,
  so their stack traces show renamed classes. Each release keeps R8's
  mappings (`terpoir-foss-<version>-mapping.txt`, and the Play one) as
  release assets; `play-upload.sh` also gives Play its mapping. Decode a trace with the Android build-tools' `retrace`:
  `retrace terpoir-foss-<version>-mapping.txt trace.txt`.

- **Signature schemes:** minSdk is 30, so `apksigner` signs with APK
  Signature Scheme v3 only. v1/v2 aren't needed, and v4 (`.idsig` files,
  for incremental installs) is turned off.
- **Signature only:** the signed APK is CI's unsigned APK plus the
  signature, so F-Droid can check its own build against it.
- **Tested:** the script was run end to end against SoftHSM, a software
  PKCS#11 token holding keys under the YubiKey's slot labels: signing,
  verification, and refusing a certificate that doesn't match the pin.
  It has not yet run against a YubiKey.
