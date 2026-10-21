<!--
SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
Copyright (C) 2026 The Terpoir Authors
See LICENSING.md for the additional permission under GPLv3 section 7.
-->

# Offline strain catalog data

`strains-kushy.tsv` is built by `make_strains.py` from Kushy's strain table
(https://github.com/kushyapp/cannabis-dataset, commit
`8b807c3dfc0aaa1e217fba4924b26ded0774d5aa`,
`Dataset/Strains/strains-kushy_api.2017-11-14.csv`), which is MIT-licensed:
see `LICENSE-kushy.txt`, shipped in the app beside the other notices.

Only the factual columns are kept: name, type, breeder and parents
(maintainer, 2026-10-21; `docs/research/strain-lookups.md`). The upstream
sources of Kushy's data are not named, and its own site is gone, so the
descriptions, effects, ailments, flavors, terpenes and cannabinoid figures
are left out.

To rebuild it:

```sh
git clone https://github.com/kushyapp/cannabis-dataset /tmp/kushy
git -C /tmp/kushy checkout 8b807c3dfc0aaa1e217fba4924b26ded0774d5aa
python3 -I make_strains.py /tmp/kushy/Dataset/Strains/strains-kushy_api.2017-11-14.csv > strains-kushy.tsv
```

The data stays as it is between rebuilds; corrections belong in the user's
own journal, where a suggestion is only ever a suggestion.
