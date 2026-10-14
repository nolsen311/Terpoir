<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Follow-ups

Items waiting on the maintainer. See CLAUDE.md, "Workflow".

## Recorded exceptions

| Exception | Closes when | Recorded |
|---|---|---|
| Tesseract4Android's JitPack native libraries log fixed status lines (no content) in release builds, which R8 cannot strip. See `docs/design-decisions.md`, "Logs and crash reports". | Every release build uses the from-source libraries (`android/ocr-native/`, logging compiled out). Done for the `foss` flavor with `-Pterpoir.ocr.aar` (F-Droid); still open for Play and GitHub Releases builds, which use JitPack. | 2026-10-11, accepted by the maintainer |
| ML Kit's usage metrics on Play leave from the app, not through the network gateway, while "Google text recognition" is on. Disclosed on its card. | Accepted as permanent by the maintainer; reopen if ML Kit offers an opt-out. | 2026-10-12, maintainer decision |
| The default map depends on OpenFreeMap's public instance, run by one person on donations with no SLA. | Accepted risk (maintainer); a user-entered tile server URL is a possible later addition. | 2026-10-12, accepted |
| F-Droid anti-features for the map: OpenFreeMap is free software and MapTiler needs the user's own key, so no NonFreeNet is expected. | Confirmed in the F-Droid recipe review. | 2026-10-12, open |
| Google Maps keys: builds now read a debug key (`GOOGLE_MAPS_API_KEY_DEBUG`, the CI secret) and a release key (`GOOGLE_MAPS_API_KEY_RELEASE` / `terpoir.googleMapsReleaseKey`); the old single `GOOGLE_MAPS_API_KEY` is no longer read. The existing key is restricted to Maps SDK for Android only. | The debug key is restricted to `app.terpoir` plus the CI debug SHA-1 (`66:A1:85:6B:FA:4D:DB:0D:44:D7:F9:B9:FB:C9:3D:DC:36:0B:C6:34`) and the local debug SHA-1, and stored as the `GOOGLE_MAPS_API_KEY_DEBUG` secret; the release key is restricted to the release (YubiKey) and Play App Signing SHA-1s and kept out of CI; the old secret is deleted. Planned to be done by Terraform in Terpoir-infra. | 2026-10-12; mostly done 2026-10-14: Terpoir-infra applied both keys (#12: the release key allows the YubiKey and Play App Signing SHA-1s; the debug key the CI SHA-1), and the hand-made key and old secret are deleted. Open: the local debug SHA-1 isn't in `maps_debug_sha1s` yet, so local debug builds don't draw the Google map |
| The website's tip jar (`site/index.html`, `#tip`) shows "Tip jar opens soon" instead of a donation link; no donation account exists yet. | The maintainer sets up GitHub Sponsors and/or Liberapay and the placeholder is replaced with the real link(s). | 2026-10-12, open (maintainer) |
| The project's MapTiler key is superseded (builds carry no map key); the CI secret `TERPOIR_MAPTILER_KEY` was deleted on 2026-10-13. | The maintainer revokes the key in the MapTiler account. | 2026-10-13, open (maintainer) |
| Google Maps costs (maintainer decision 2026-10-13: keep Google Maps, guarded): as the app uses it (a basic map in the Maps SDK for Android, no map ID, no Places, Geocoding or Directions), map loads are billed under the "Maps SDK" SKU (formerly "Mobile Native Dynamic Maps"): free usage cap Unlimited, no price at any volume (Google's pricing page, read 2026-10-13). MapTiler is only ever the user's own key, so maps cost the project nothing at any scale. The guard is a $1 budget alert on the Maps project in Terpoir-infra (D6), which the Maps keys can't be created without. | Terpoir-infra's budget alert is applied (B7). | 2026-10-13; closed 2026-10-14: the $1 budget alert "Terpoir Maps: any spend" is applied |
| No device test yet that the app opens no network connection with the default settings (`docs/design-decisions.md`, "Android project"). The gateway's unit tests cover the refusal logic. | An instrumented test drives the app with the defaults (opening the spot editor included) and finds no traffic from its UID. | 2026-10-12, open |

## Blocked network sources

When a source could not be reached, what was done instead stays here until
the maintainer **allows the domain** or **accepts the alternative**. Close
an item by moving it to "Resolved" with the decision and date.

### Open

| Host | Needed for | Done instead | Decision needed |
|---|---|---|---|
| GitHub source archives (`github.com/<repo>/archive/…`, served from `codeload.github.com`) of `tesseract-ocr/tesseract` and `adaptech-cz/Tesseract4Android`. The session proxy serves `git clone` of public repositories but refuses their archive downloads. | The two pinned tarballs in `android/ocr-native/sources.txt`, to build the from-source OCR AAR locally and test the `terpoir.ocr.aar` switch | Cloned each pinned tag and recreated the tarball with `git archive` + `gzip -n`. Both are byte-identical to the SHA-256 pinned in `sources.txt` (`build.sh` checks again), and the AAR built from them matches `android/ocr-native/SHA256SUMS`. CI's OCR workflow downloads the real archives. | Accept the recreated, hash-verified tarballs for local builds, or widen this session's GitHub access so the archives download directly. |

### Resolved

| Host | Needed for | Decision |
|---|---|---|
| `play.google`, `google.play` (where `play.google.com/about/…` policy pages now redirect) | Google Play's Developer Program Policy pages | Domains allowed (2026-10-13). The Policy Center (`play.google/developer-content-policy/`) links each policy to its Play Console Help Center article, the copies already read; `docs/play-listing.md` now cites the Policy Center's link for Health Content and Services (`answer/9878878`). |
| `support.google.com`, `play.google.com` | The Play Console Help Center, for the wording in `docs/play-listing.md` | Domains allowed (2026-10-13). Every section was checked against the Help Center and linked. Changes: the required medical-device disclaimer in the full description; Restrict Minor Access; "App access" is now "Sign-in details"; the "not shared" reasoning rests on Google's disclosure-and-consent exception; the foreground-service use case; the 12-tester closed test. Three in-app gaps went to `docs/roadmap.md`. The IARC questions aren't published, so they're confirmed in the Console. |
| `openfreemap.org`, `www.cloudflare.com`, `policies.google.com` | Checking the links in `site/privacy.html` | Domains allowed (2026-10-13). All three links answer; OpenFreeMap has a privacy policy (`/privacy/`), now linked instead of its home page. |
| `developers.google.com` (did not resolve from the session) | Google Maps Platform pricing and the Maps SDK for Android billing page, for the cost decision | Domain allowed (2026-10-13). Until then the pricing was read through search summaries; the live pages confirm them: the "Maps SDK" SKU is unlimited and free. |
| `tiles.openfreemap.org` | OpenFreeMap's live style and TileJSON, to list every host the default map loads | Domain allowed (2026-10-13). The live "Liberty" style (style, vector tiles, glyphs, sprites, shaded relief) and the TileJSON's tile URLs (`/planet/<version>/{z}/{x}/{y}.pbf`) are all on `tiles.openfreemap.org`, with no redirects; that is the only host `MapProvider.OPENFREEMAP` allows. Closes the earlier exception about other font or sprite hosts. |
| `api.maptiler.com`, `docs.maptiler.com`, `www.maptiler.com` | MapTiler's style URL, key restrictions and terms, for the online map | Domains allowed. The style URL form is confirmed (the API answers "Missing key" / "Invalid key" without a valid one). Key restriction: "Allowed User-Agent header" is a case-sensitive substring match, so `Terpoir` works; no HTTP origins for an app. Terms: the map must always show "© MapTiler © OpenStreetMap contributors" (now drawn on the map), and the Free plan is for non-commercial use and R&D only and shows MapTiler's logo (see "Recorded exceptions"). Superseded the same day: MapTiler is now only used with the user's own key, so the project's plan and its terms no longer apply. (2026-10-12) |
| `jitpack.io` | Tesseract4Android, the only published build of it | Domain allowed. The placeholder "no text engine" stub was replaced with Tesseract. (2026-10-11) |
| `huggingface.co` | The license of the `ocrs` OCR model weights | Alternative accepted: Tesseract instead of `ocrs`, so the weights are not used. (2026-10-11) |
| `developers.google.com` | Google's ML Kit data disclosure page | Domain allowed. Read directly: ML Kit's Android SDKs collect device information, app information and performance metrics for diagnostics and usage analytics, as the search summaries said. ML Kit is not in the scanner yet; per #33 it becomes a Play-only opt-in through Play services, with that disclosure. (2026-10-11) |
| `www.ijg.org` | libjpeg 9f release, for the IJG license notice | Domain allowed. `jpegsrc.v9f.tar.gz` from `www.ijg.org` is byte-identical to the copy from `ijg.org` (SHA-256 `04705c110cb2469caa79fb71fba3d7bf834914706e9641a4589485c1f832565b`). (2026-10-11) |
