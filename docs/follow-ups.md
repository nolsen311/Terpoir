<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Follow-ups

Items waiting on the maintainer. See CLAUDE.md, "Workflow".

## Blocked network sources

When a source could not be reached, what was done instead stays here until
the maintainer **allows the domain** or **accepts the alternative**. Close
an item by moving it to "Resolved" with the decision and date.

### Open

| Host | Needed for | Done instead | Decision needed |
|---|---|---|---|
| `developers.google.com` | Google's ML Kit data disclosure page, to check what ML Kit sends Google | Relied on search-result summaries of that page and of ML Kit's terms. ML Kit was then left out of the app entirely, so nothing shipped depends on it. | Accept the summaries as enough, or allow the domain so the page is read directly before ML Kit is reconsidered. |
| `www.ijg.org` | libjpeg 9f release, for the IJG license notice in `android/app/src/main/assets/licenses/libjpeg-IJG.txt` | Fetched the same release from `ijg.org` (no `www`), which is allowed; it matches the copy vendored in Tesseract4Android byte for byte. | Accept `ijg.org` as the source, or also allow `www.ijg.org`. |

### Resolved

| Host | Needed for | Decision (2026-10-11) |
|---|---|---|
| `jitpack.io` | Tesseract4Android, the only published build of it | Domain allowed. The placeholder "no text engine" stub was replaced with Tesseract. |
| `huggingface.co` | The license of the `ocrs` OCR model weights | Alternative accepted: Tesseract instead of `ocrs`, so the weights are not used. |
