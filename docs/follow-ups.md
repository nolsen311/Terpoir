<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Follow-ups

Items waiting on the maintainer. See CLAUDE.md, "Workflow".

## Recorded exceptions

| Exception | Closes when | Recorded |
|---|---|---|
| Tesseract4Android's prebuilt native libraries log fixed status lines (no content) in release builds, which R8 cannot strip. See `docs/design-decisions.md`, "Logs and crash reports". | Tesseract and Leptonica are built from source with logging compiled out, and the FOSS and Play builds use those libraries. | 2026-10-11, accepted by the maintainer |

## Blocked network sources

When a source could not be reached, what was done instead stays here until
the maintainer **allows the domain** or **accepts the alternative**. Close
an item by moving it to "Resolved" with the decision and date.

### Open

None.

### Resolved

| Host | Needed for | Decision (2026-10-11) |
|---|---|---|
| `jitpack.io` | Tesseract4Android, the only published build of it | Domain allowed. The placeholder "no text engine" stub was replaced with Tesseract. |
| `huggingface.co` | The license of the `ocrs` OCR model weights | Alternative accepted: Tesseract instead of `ocrs`, so the weights are not used. |
| `developers.google.com` | Google's ML Kit data disclosure page | Domain allowed. Read directly: ML Kit's Android SDKs collect device information, app information and performance metrics for diagnostics and usage analytics, as the search summaries said. ML Kit stays out of the app. |
| `www.ijg.org` | libjpeg 9f release, for the IJG license notice | Domain allowed. `jpegsrc.v9f.tar.gz` from `www.ijg.org` is byte-identical to the copy from `ijg.org` (SHA-256 `04705c110cb2469caa79fb71fba3d7bf834914706e9641a4589485c1f832565b`). |
