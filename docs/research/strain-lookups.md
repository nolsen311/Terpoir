<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Strain lookups: data source research

For the roadmap item "Strain lookups, 0.6.0". The hosts this research could
not reach are listed in `docs/follow-ups.md`, "Blocked network sources".

Researched 2026-10-14. **Read this caveat first:** the session's network proxy
refused (HTTP 403 on CONNECT) every provider host except GitHub. Leafly,
SeedFinder, The Strain API, Otreeba, Kaggle, Wikidata, Hugging Face,
cannlytics.com, kushy.net, vdb.openthc.org and web.archive.org could not be read.
Findings for those rest on GitHub-hosted code or on search-engine snippets, and
are marked **unverified**. None of their terms are quoted here, because none
could be read first-hand.

## Summary and recommendation

No source I could verify gives strain lineage, terpenes and effects under a
licence that clearly allows redistribution. The only fully verified open dataset
is the **Kushy dataset** (MIT, 9,524 strain rows). It is frozen since
2017-11-14 and mostly lists only name, type and breeder. **OpenTHC VDB**
(GPL-3.0 code, names and types only) is a good name list to check against, but
its data licence is unstated. Recommendation:

- **Offline catalog:** ship Kushy's strain table as v1 (MIT notice in the app's
  licences screen), limited to name, type, breeder and the few filled crosses
  and cannabinoid fields. Re-check provenance before shipping (see Kushy below).
  Evaluate **Wikidata (CC0)** as the long-term, refreshable source once
  `wikidata.org` is reachable.
- **Online toggle:** **SeedFinder** is the best lineage source, but only as a
  per-user key toggle. Each user registers and verifies their own key or IP,
  and the key travels in the URL. Its terms are unverified, so hold until
  someone reads them.
- **Rule out for now:** Leafly (no public API; partner, retailer and academic
  access only; terms unread), Kaggle Leafly scrapes (provenance taints any
  licence label), The Strain API and Otreeba (likely dead or keyed; unverified),
  Cannlytics (lab-result samples rather than a strain catalog; data licence
  unstated).

## Comparison

| Candidate | Access | Caching / redistribution | Cost / key | What leaves the phone | Fields | Fit |
|---|---|---|---|---|---|---|
| Kushy dataset | Bulk CSV/SQL on GitHub; last commit 2017-11-14 | MIT: "use, copy, modify, merge, publish, distribute" (verified) | Free, no key | Nothing (offline) | name, type, breeder, crosses (sparse), effects/ailment/flavor (~11%), THC/CBD (sparse) | **Offline catalog (v1)** |
| OpenTHC VDB | Site has SQL/JSON/CSV/TSV/XML downloads (per code); last code commit 2025-05-29 | Code GPL-3.0; data licence only "an open license" (aspiration), images "CCA" | Free | Nothing if bundled | ID, type, name only | Offline name list, after licence confirmation |
| Wikidata | SPARQL / dumps | CC0 (well known; **not verified this session**) | Free, no key | Online: query + IP to Wikimedia | Unknown strain coverage | Candidate offline source; unverified |
| SeedFinder.eu | JSON API (`/api/json/search.json`, `strain.json`); wrapper last updated 2022-05-01 | **Unverified**, terms unreadable | Account plus verified domain/IP or key per user | Search text, user's key (`&ac=` in URL), IP | name, breeder, parents, hybrids, medical, reviews, tasting (per wrapper) | Online toggle (per-user key); hold pending terms |
| Leafly | No public strain API (per secondary sources) | **Unverified** | Partner or retailer key only | n/a | Rich (terpenes, effects, lab data) | Unusable |
| Kaggle Leafly scrapes | Bulk download | Unverified; derived from Leafly content | Kaggle account | n/a | Leafly fields | Unusable (provenance) |
| The Strain API (evanbusse) | API (status unknown) | Unverified | Unverified | Query, IP | Unverified | Unverified; likely unusable |
| Otreeba / Open Cannabis API | API, key required (secondary) | Unverified | Project key, a problem in open source | Query, IP, key | strains, products | Unusable as described |
| Cannlytics | Python lib; Parquet results product; Firebase | Code MIT; data licence not stated in repo | Free | n/a | Lab results per sample; "Strains 5,000+" in Firebase | Not a catalog; maybe later for terpene stats |

## Per candidate

### Kushy dataset (verified)
- Repo: https://github.com/kushyapp/cannabis-dataset. Two commits, both on
  2017-11-14 ("1.0", "Fixing links"). The data is not maintained.
- Licence: https://github.com/kushyapp/cannabis-dataset/blob/master/LICENSE is
  MIT, "Copyright (c) 2016-present, Kushy". The decisive clause: *"Permission
  is hereby granted, free of charge, to any person obtaining a copy ... to
  use, copy, modify, merge, publish, distribute, sublicense, and/or sell
  copies ... subject to the following conditions: The above copyright notice
  and this permission notice shall be included in all copies or substantial
  portions."* The README says "This dataset is MIT licensed. *Go crazy.*"
  MIT is GPL-compatible and F-Droid-safe. Attribution is the only
  requirement.
- Data, from my count of `Dataset/Strains/strains-kushy_api.2017-11-14.csv`:
  9,524 rows. Fields that are non-empty and non-zero: type 9,500; breeder
  8,979; effects 1,015; THC 764; crosses (parent IDs) 162; CBD 83;
  description 1; terpenes 1. The CSV also has columns for many minor
  cannabinoids, mostly empty.
- Provenance caveat: the README does not name the upstream sources. The
  effect and ailment vocabulary ("Dry Mouth, Paranoid", "Lack of Appetite") is
  the style used by commercial strain sites. Before shipping, consider
  dropping the effects/ailment/flavor/description columns and keeping only
  the factual fields (name, type, breeder, crosses).

### OpenTHC Variety Database (code verified, data unverified)
- Repo: https://github.com/openthc/vdb, licensed GPL-3.0 (LICENSE file). Last
  commit 2025-05-29 ("update default home page").
- The README says: *"This code and data is provided for others who are in the
  cannabis software space to access a directory of variety names."* The
  repository itself contains no data. The site code (`lib/Controller/Download.php`)
  serves `/download/strains.{sql,json,csv,tsv,xml}`, and the CSV export has
  three columns: ID, Type, Strain.
- Licence wording on the site's templates: *"An open license on the data to
  keep it available for everyone."* (`view/home.php`, a stated goal) and
  *"All images become part and property of our Open Cannabis Varietey Dataset
  which is distributed under a CCA license."* (`view/single.php`). No specific
  licence for the data is named. Ask OpenTHC before shipping it.
- The vdb.openthc.org host is blocked, so I could not check size or currency.
  Search snippets claim "3,000+" varieties (unverified).

### Wikidata (unverified)
- Hosts `www.wikidata.org` and `query.wikidata.org` are blocked. Wikidata's
  structured data is published under CC0, but I could not confirm that this
  session or check how many strain items exist and which properties they
  carry (parents, breeder). Being CC0 and refreshable makes it the best
  long-term open candidate. It also works as an opt-in online toggle; the
  disclosure would cover query text and IP sent to the Wikimedia Foundation.

### SeedFinder.eu (unverified terms)
- Hosts `en.seedfinder.eu` and `seedfinder.eu` are blocked. Evidence comes
  from the MIT-licensed community wrapper https://github.com/hdb/seedfinder-python
  (last commit 2022-05-01). Its `docs/source/setup.md` says: *"You will need
  to create an account at seedfinder.eu and verify a domain or IP in order to
  access the API."* The client appends the key as a URL query parameter
  (`'&ac={}'.format(api_key)`) to `https://en.seedfinder.eu/api/json/search.json?q=...`.
  Endpoints include search, strain info with parents and hybrids, medical
  info, reviews and tasting notes, and breeder IDs.
- Fit: a project key embedded in an open-source APK is not workable. A
  per-user key, where each user registers and pastes their own key, can work
  behind a toggle. The disclosure would cover strain name, the user's key and
  IP sent to seedfinder.eu. The API terms page
  (`https://en.seedfinder.eu/userarea/action/jsonapi.html`) still needs
  reading, especially on caching results in the user's journal. An API that
  needs registration may also earn the F-Droid "NonFreeNet" flag; that is
  acceptable only as an optional toggle.
- Conflict with a recorded decision: "Catalog lookups" in
  `docs/design-decisions.md` says "app-level API keys (no per-user
  identifiers)". A per-user SeedFinder key is a per-user identifier sent with
  every lookup, so this option needs the maintainer to amend that rule first
  (for example: a key the user brings, named on the disclosure card).

### Leafly (unverified)
- Leafly's own pages (`www.leafly.com`, `leafly.com`, `developer.leafly.com`,
  `consumer-api.leafly.com`) are blocked. Secondary sources only:
  - New Cannabis Ventures reports that Leafly "locked out developers ... from
    its API" in February 2016
    (https://www.newcannabisventures.com/leafly-api-cannabis-reports/).
  - apis.io (https://apis.io/providers/leafly/) describes partner-only menu
    and order APIs with no self-serve consumer data API.
  - Leafly's research programme gives strain lab data only to accredited
    researchers.
- Third-party scrapers (Apify) hit an undocumented `consumer-api.leafly.com`.
  Using that from the app would be unsanctioned and fragile.
- Leafly's ToS is at https://www.leafly.com/company/tos but I could not read
  it, so no clause is quoted. Treat Leafly as unusable unless the maintainer
  obtains written permission.

### Kaggle cannabis strain datasets (unverified)
- `www.kaggle.com` is blocked. The widely circulated Kaggle strain sets are
  described in search results as scraped from Leafly. Whatever licence label
  the uploader chose cannot grant rights the uploader did not hold. Rule out.

### The Strain API (strainapi.evanbusse.com), unverified
- Host blocked, and search found no docs or repo. I could not determine
  liveness, key model or terms. Rule out until someone can read it.

### Otreeba / Open Cannabis API (unverified)
- `otreeba.com` and `api.otreeba.com` are blocked. Secondary sources
  (publicapis.io, RapidAPI blog) say an API key is required and the service
  is tied to a dispensary's commercial data. A per-project key cannot ship in
  an open-source app. Rule out.

### Cannlytics (code verified, data licence unverified)
- Repo: https://github.com/cannlytics/cannlytics. The code is MIT (LICENSE).
  The latest release is "Release 1.0.5" on 2026-09-30, so the project is
  active.
- The README's "Data Assets" table lists "Strains | 5,000+ | With
  terpene/cannabinoid statistics" and "Lab Results | 995,000+". Strains are
  read from Cannlytics' Firebase (`get_document('public/data/strains/blue-dream')`).
  Results are a Parquet product of per-sample lab results (`cannlytics/datasets.py`).
- The repo states no data licence. The Hugging Face dataset cards
  (`huggingface.co/datasets/cannlytics/...`) are blocked. A search snippet
  says the results card restricts commercial use of individuals' contact
  data. Not a strain catalog; revisit later for terpene and cannabinoid
  averages if the data licence turns out to be open.

## Blocked or unverified

Proxy returned `CONNECT tunnel failed, response 403` for these hosts:
`www.leafly.com`, `leafly.com`, `developer.leafly.com`, `consumer-api.leafly.com`,
`seedfinder.eu`, `en.seedfinder.eu`, `strainapi.evanbusse.com`, `otreeba.com`,
`api.otreeba.com`, `www.kaggle.com`, `www.wikidata.org`, `query.wikidata.org`,
`huggingface.co`, `cannlytics.com`, `kushy.net`, `opencannabis.info`,
`www.cannabisreports.com`, `web.archive.org`, `vdb.openthc.org`.
Reachable: `github.com`, `raw.githubusercontent.com` (git clone and raw files),
and search-engine snippets.

Unverified as a result: all Leafly, SeedFinder, Strain API, Otreeba, Kaggle and
Wikidata terms; the data licences for OpenTHC VDB and Cannlytics.
