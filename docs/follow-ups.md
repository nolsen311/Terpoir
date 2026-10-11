<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Follow-ups

Items waiting on the maintainer. See CLAUDE.md, "Workflow".

## Recorded exceptions

| Exception | Closes when | Recorded |
|---|---|---|
| Tesseract4Android's JitPack native libraries log fixed status lines (no content) in release builds, which R8 cannot strip. See `docs/design-decisions.md`, "Logs and crash reports". | Every release build uses the from-source libraries (`android/ocr-native/`, logging compiled out). Done for the `foss` flavor with `-Pterpoir.ocr.aar` (F-Droid); still open for Play and GitHub Releases builds, which use JitPack. | 2026-10-11, accepted by the maintainer |

## Blocked network sources

When a source could not be reached, what was done instead stays here until
the maintainer **allows the domain** or **accepts the alternative**. Close
an item by moving it to "Resolved" with the decision and date.

### Open

| Host | Needed for | Done instead | Decision needed |
|---|---|---|---|
| GitHub source archives (`github.com/<repo>/archive/…`, served from `codeload.github.com`) of `tesseract-ocr/tesseract` and `adaptech-cz/Tesseract4Android`. The session proxy serves `git clone` of public repositories but refuses their archive downloads. | The two pinned tarballs in `android/ocr-native/sources.txt`, to build the from-source OCR AAR locally and test the `terpoir.ocr.aar` switch | Cloned each pinned tag and recreated the tarball with `git archive` + `gzip -n`. Both are byte-identical to the SHA-256 pinned in `sources.txt` (`build.sh` checks again), and the AAR built from them matches `android/ocr-native/SHA256SUMS`. CI's OCR workflow downloads the real archives. | Accept the recreated, hash-verified tarballs for local builds, or widen this session's GitHub access so the archives download directly. |

### Resolved

| Host | Needed for | Decision (2026-10-11) |
|---|---|---|
| `jitpack.io` | Tesseract4Android, the only published build of it | Domain allowed. The placeholder "no text engine" stub was replaced with Tesseract. |
| `huggingface.co` | The license of the `ocrs` OCR model weights | Alternative accepted: Tesseract instead of `ocrs`, so the weights are not used. |
| `developers.google.com` | Google's ML Kit data disclosure page | Domain allowed. Read directly: ML Kit's Android SDKs collect device information, app information and performance metrics for diagnostics and usage analytics, as the search summaries said. ML Kit is not in the scanner yet; per #33 it becomes a Play-only opt-in through Play services, with that disclosure. |
| `www.ijg.org` | libjpeg 9f release, for the IJG license notice | Domain allowed. `jpegsrc.v9f.tar.gz` from `www.ijg.org` is byte-identical to the copy from `ijg.org` (SHA-256 `04705c110cb2469caa79fb71fba3d7bf834914706e9641a4589485c1f832565b`). |
