<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Strain lookups: data source research

For the roadmap item "Strain lookups, 0.7.0" (planned as 0.6.0 when written). The hosts this research could
not reach are listed in `docs/follow-ups.md`, "Blocked network sources".

Researched 2026-10-14. **Read this caveat first:** the session's network proxy
refused (HTTP 403 on CONNECT) every provider host except GitHub. Leafly,
SeedFinder, The Strain API, Otreeba, Kaggle, Wikidata, Hugging Face,
cannlytics.com, kushy.net, vdb.openthc.org and web.archive.org could not be read.
Findings for those rest on GitHub-hosted code or on search-engine snippets, and
are marked **unverified**. None of their terms are quoted here, because none
could be read first-hand.

**Update 2026-10-15:** the maintainer allowed `seedfinder.eu`,
`en.seedfinder.eu`, `kannapedia.net`, `www.allbud.com` and the TerpHunter hosts.
SeedFinder, Kannapedia and AllBud were read first-hand that day; their sections
below quote their own pages. TerpHunter's apex `terphunter.com` was refused at
first and allowed later that day; its section also quotes its own pages.
`medicinalgenomics.com` (Kannapedia's owner) was refused at first, then
allowed and read the same day.

## Summary and recommendation

No source gives strain lineage, terpenes and effects under terms that allow an
app to use them. The four sites checked on 2026-10-15 change nothing for the
offline catalog, and remove the one online candidate:

- **SeedFinder's JSON API is gone** ("discontinued per july 1st, 2024"). Its
  legal notice allows only personal, non-commercial use and forbids
  reproduction or distribution. No live lookup, cache or bundle.
- **AllBud** forbids any "robot, spider or other automatic device" and any
  storing or redistribution. Its API exists but answers 401 without an
  account. Its data is mostly descriptions and user reviews. Rule out.
- **Kannapedia** is real lab data (DNA-based chemotype, registrant-supplied
  cannabinoids and terpenes), but for only 1,462 public samples, with no
  parents, no API, no stated licence and no terms page. Only usable with
  written permission from Medicinal Genomics.
- **TerpHunter** has the right fields (parents, chemotype, terpene profile),
  but no public API yet ("Pro — Coming later"); its own API needs a login,
  and its terms forbid scraping and redistribution. Its maintainer answered
  the permission request (2026-10-20) with personal Pro access through an MCP
  endpoint, not yet with permission for the app; see "Reply" below.

Recommendation; the maintainer picks one:

1. **Offline only (recommended).** Ship Kushy's strain table as v1 (MIT notice
   in the app's licences screen), limited to name, type, breeder and the few
   filled crosses and cannabinoid fields. Re-check provenance before shipping
   (see Kushy below). Evaluate **Wikidata (CC0)** as the long-term, refreshable
   source once `wikidata.org` is reachable. No online toggle in the first lookups release.
2. **Option 1 plus a link out.** A "Look up on the web" action opens the
   user's browser on a site they pick (SeedFinder, Kannapedia, AllBud). The
   app fetches nothing and fills nothing, so no site's terms on apps apply.
   It still needs a disclosure card: the strain name goes to that site, which
   runs its own trackers in the browser (see each section).
   **Saved sites (maintainer, 2026-10-15):** the app remembers the user's
   lookup sites, so they pick from a saved list instead of re-entering one
   each time. They can add, edit and remove sites; a site is a URL with a
   placeholder for the strain name. The list is kept inside the encrypted
   journal, because it reveals interests, and is never sent anywhere. The
   toggle's off-action deletes the saved list.
3. **Option 1, and ask.** Write to Medicinal Genomics for permission to bundle
   Kannapedia's chemotype and registrant chemistry, and to SeedFinder
   (Snorkel Spain SL) for its lineage data. Build nothing until a written
   licence arrives.

Also ruled out: Leafly (no public API; partner, retailer and academic access
only; terms unread), Kaggle Leafly scrapes (provenance taints any licence
label), The Strain API and Otreeba (likely dead or keyed; unverified),
Cannlytics (lab-result samples rather than a strain catalog; data licence
unstated).

## Comparison

| Candidate | Access | Caching / redistribution | Cost / key | What leaves the phone | Fields | Fit |
|---|---|---|---|---|---|---|
| Kushy dataset | Bulk CSV/SQL on GitHub; last commit 2017-11-14 | MIT: "use, copy, modify, merge, publish, distribute" (verified) | Free, no key | Nothing (offline) | name, type, breeder, crosses (sparse), effects/ailment/flavor (~11%), THC/CBD (sparse) | **Offline catalog (v1)** |
| OpenTHC VDB | Site has SQL/JSON/CSV/TSV/XML downloads (per code); last code commit 2025-05-29 | Code GPL-3.0; data licence only "an open license" (aspiration), images "CCA" | Free | Nothing if bundled | ID, type, name only | Offline name list, after licence confirmation |
| Wikidata | SPARQL / dumps | CC0 (well known; **not verified this session**) | Free, no key | Online: query + IP to Wikimedia | Unknown strain coverage | Candidate offline source; unverified |
| SeedFinder.eu | **No API**: "discontinued per july 1st, 2024"; HTML pages only | "personal or private use; commercial use is prohibited"; no reproduction or distribution (verified) | n/a | n/a (pages load Google Analytics, Microsoft Clarity) | ~42,000 strain pages, 2,146 breeders: type, breeder, flowering time, parents, hybrids, user effects/aroma/taste; lab cannabinoids for 91 varieties; no terpenes | Unusable without written permission |
| Kannapedia | **No API**; one 2.8 MB HTML list, filtered in the browser; VCF downloads | No terms of use on either Kannapedia or its owner's site; owner's footer: "Copyright all right reserved" (verified) | Free to view | n/a (Google Analytics only after consent) | 1,462 public samples (2,418 registered): chemotype from DNA, genetic relatives by distance, gene variants incl. terpene synthases; registrant cannabinoids and terpenes (sparse); no parents | Unusable without written permission |
| AllBud | Internal API (`/api/`) answers 401; HTML pages | "personal, non-commercial use only"; no robots or storing (verified) | Account (not offered to third parties) | n/a (Google Analytics, ad network, ShareThis) | ~16,000 strains: type with sativa/indica %, THC/CBD/CBN, description, effects, "may relieve", flavors, aromas, reviews; no terpenes or parent fields | Unusable |
| TerpHunter | No public API: "Pro — Coming later: Full API access ... Bulk data export"; site API answers 401 without a login | No "scrape, crawl, or bulk-download"; no right "to redistribute our aggregated data" (verified) | Free account (email), not shareable | Query, account, IP (first-party page-view stats only) | 5,200+ strains (parents, chemotype, terpene profile, partly AI-made); 8,000+ lab-tested products in MI, OH, MO | Not usable in the app yet: the maintainer has personal Pro access for evaluation; bundling, per-user lookups and sources asked again (2026-10-21) |
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

### SeedFinder.eu (verified 2026-10-15)
- Owner: SNORKEL SPAIN, SL, Montornès del Vallès, Spain (legal notice). The
  site was rebuilt; `en.seedfinder.eu` now redirects to `seedfinder.eu/en`.
- **No API.** The home page (https://seedfinder.eu/en) says: *"JSON Api — The
  JSON api has been discontinued per july 1st, 2024"*. The old endpoints
  (`/api/json/search.json`, `/api/json/strain.json`) and the API page
  (`/userarea/action/jsonapi.html`) return 404 on both hosts. The
  `hdb/seedfinder-python` wrapper used in the first round is dead.
- **Terms**, from the legal notice (https://seedfinder.eu/en/legal-notice):
  - *"The intellectual and industrial property rights of this website,
    including its source code, design, photographs, texts, navigation
    structure, databases ... belong to SeedFinder."*
  - *"Users are not allowed to reproduce, distribute, or commercially use the
    website's content, or create derivative works, without prior consent from
    SeedFinder."*
  - *"Users may view, print, and partially download website content only
    under the following conditions: ... Content is obtained exclusively for
    personal or private use; commercial use is prohibited."*
  - So: (a) live lookups by an app would mean scraping HTML, which no clause
    allows; (b) caching fetched content in the journal is "partially
    download[ing]", arguably personal, but done by our app for every user;
    (c) bundling an offline copy is reproduction and distribution: forbidden.
    Terpoir being free does not help: the clause forbids distribution as
    such, not only sale.
- **Data** (sample: https://seedfinder.eu/en/strain-info/white-widow/dutch-passion):
  type ("mostly indica"), breeder, indoor/outdoor, flowering time, the
  breeder's description, awards, parents and a lineage tree, hybrids
  (descendants), user reviews, user effect, aroma and taste tags, medical
  reports, photos. Cannabinoids come from lab tests on few strains: the
  cannabinoids page compares with *"the average of all 91 tested cannabis
  varieties"*. No terpene data.
- **Coverage:** the sitemap (https://seedfinder.eu/sitemap/sitemap-index.xml,
  generated 2026-10-15) lists 41,990 strain pages (one per breeder's version)
  and 2,146 breeder pages.
- **Provenance:** breeder claims (descriptions, parents) and user uploads
  (reviews, tags). Lineage is the site's strength; it is curated, not lab data.
- **Privacy:** pages load Google Analytics (`G-F22HDGZ6S0`), Microsoft Clarity
  and Sentry, behind Cloudflare. The privacy policy says it collects *"IP
  addresses, browser type, and site usage"*. An app fetching HTML would not
  run those scripts, but would send the query and IP to Cloudflare and
  SeedFinder.
- **Fit:** none without written consent. The per-user key idea, and its
  conflict with "app-level API keys (no per-user identifiers)" in
  `docs/design-decisions.md`, are moot now that the API is gone. A link out
  to the user's browser stays possible (option 2 above).

### Kannapedia (verified 2026-10-15)
- Owner: *"Kannapedia® is a property of Medicinal Genomics"* (Medicinal
  Genomics Corp., Beverly, MA; https://kannapedia.net/contact). Registrants pay
  to sequence a sample; the report is published and its hash stamped on the
  Bitcoin blockchain to establish prior art.
- **Coverage:** the home page (https://kannapedia.net/) says *"2418 total
  registered strains"*; the public list (https://kannapedia.net/strains)
  shows *"1462 strains found"*. These are samples, not canonical strains:
  "Durban Poison" appears as several RSP numbers, and some names are codes
  ("RKM-2018-025", "ST").
- **Data per report** (sample: https://kannapedia.net/strains/rsp11014,
  Durban Poison):
  - General: name, RSP number, registrant, accession date, reported sex.
  - **Chemotype from DNA:** "Plant Type: Type I" with THCAS, CBDAS and CBCAS
    called "Intact" or "Deleted". Type I/II/III maps to THC-dominant,
    balanced, CBD-dominant. This is lab data and the most reliable field.
  - **Chemistry:** *"Cannabinoid and terpenoid information provided by the
    registrant"*: six cannabinoid pairs and about 21 terpenes. Sparse: of
    five reports I opened, two had values (Durban Poison, Blue Dream
    RSP10612); Garlic, OG Kush (RSP10070) and one other said *"No
    information provided"*.
  - **Relatives, not parents:** "Genetic Relatives" lists the 20 nearest and
    most distant samples by genetic distance. No pedigree or parent fields.
  - Gene variants: *"predicted high-impact variants in selected cannabis
    genes"*, including terpene synthases, flowering and autoflower genes.
    Predictions, not measured terpene levels; not useful as journal fields.
  - Heterozygosity, a rarity percentile, microbiome reads, and a VCF that
    *"can be downloaded by anyone"*.
- **Access:** no API. `/api`, `/api/strains`, `/strains.json`,
  `/strains/rsp11014.json` and `/openapi.json` all return 404. The strain
  list is one 2.8 MB HTML page holding all 1,462 entries (name, registrant,
  date, RSP link); `search.js` filters it in the browser, and
  `/strains?q=blue+dream` returns the same page. robots.txt (served with a 404
  status, same text) disallows `/*?*`, the sequence downloads, and AI
  crawlers: *"AI training crawlers, excluded deliberately."*
- **Terms:** none on Kannapedia. `/terms` is 404, and no page carries a
  copyright line or licence. The privacy policy (https://kannapedia.net/privacy)
  is Medicinal Genomics' and covers only personal data.
- **Owner's site** (https://medicinalgenomics.com/, read 2026-10-15): no terms
  of use either. `/terms-of-use/`, `/terms-and-conditions/`, `/terms/`,
  `/legal/` and `/terms-of-service/` are 404, and its 370-page sitemap has no
  terms, licence or API page. The footer says *"©2026 MEDICINAL GENOMICS
  CORPS Copyright all right reserved."* Its privacy policy
  (https://medicinalgenomics.com/privacy/) also covers only personal data.
  What it says about the data:
  - StrainSEEK FAQ (https://medicinalgenomics.com/strainseek-frequently-asked-questions/):
    *"The customer has total control over access—data and analyses from
    StrainSEEK identification and registration can be made public or kept
    private."* So each report's publication is the registrant's choice;
    nothing says the public data is licensed for reuse.
  - The Kannapedia 2.0 announcement (2021-10-05,
    https://medicinalgenomics.com/introducing-kannapedia-2-0-the-worlds-most-comprehensive-cannabis-genomic-library/)
    promised *"Expanded API capabilities providing easy interoperability with
    multiple data inputs from MGC's StrainSEEK® sequencing panel and Whole
    Genome sequencing"*: an input pipeline for their own sequencing, not a
    public query API. No API documentation exists on either site.
  - Older VCF files are listed at https://medicinalgenomics.com/kannapedia-fastq/.
- So, with no licence anywhere and "all right reserved" on the owner's site:
  (a) live fetching is scraping with no permission, (b) caching and
  (c) bundling need a licence. Contact: `info@medicinalgenomics.com`
  (Kannapedia's contact page).
- **Privacy:** Google Analytics (`UA-58561949-2`) is loaded but only
  configured after consent (`enableMGCAnalytics()`); the policy says *"We do
  not partner with or have special relationships with any ad server
  companies"*. A lookup that fetched the whole list would reveal only the IP
  and timing, not the query, because the list is searched on the phone.
- **Fit:** the chemotype (Type I/II/III) and, where present, the registrant's
  cannabinoids and terpenes fit Terpoir's type, cannabinoid and terpene
  suggestions. But coverage is small, names are per sample, and lineage is
  missing. Usable only with written permission from Medicinal Genomics (ask
  for a licence to bundle name, chemotype and registrant chemistry). A link
  out to a report is fine.

### AllBud (verified 2026-10-15)
- Owner: AB Resources, LLC, Tucson, Arizona (terms, section 24).
- **Coverage:** `https://www.allbud.com/sitemap-strains.xml` lists 16,060
  strain pages.
- **Data** (sample:
  https://www.allbud.com/marijuana-strains/sativa-dominant-hybrid/blue-dream):
  type with a sativa/indica split ("60% Sativa / 40% Indica"), THC range
  ("17% 24%"), CBD, CBN, a long description (parents appear only in prose:
  "a potent cross between the hugely popular Blueberry X Haze strains"),
  effects, *"May Relieve"* conditions (ADD/ADHD, PTSD, depression ...),
  flavors, aromas, ratings and reviews. No terpene or parent fields.
- **Provenance:** editorial descriptions and user reviews; no lab sources
  named. The medical "may relieve" lists conflict with Terpoir's "never
  medical claims" wording rule.
- **Access:** no public API. `https://www.allbud.com/api/` answers with an
  internal Django REST Framework index (strains, effects, aromas, users,
  dispensaries ...); `/api/strains/varieties` returns 401. Undocumented and
  for their own apps.
- **Terms** (https://www.allbud.com/terms-of-use, *"Last updated on September
  5, 2014"*):
  - *"These Terms of Use permit you to use the Website for your personal,
    non-commercial use only. You must not reproduce, distribute, modify,
    create derivative works of, publicly display, publicly perform,
    republish, download, store or transmit any of the material on our
    Website"*, except RAM copies and the browser's own cache.
  - Prohibited: *"Use any robot, spider or other automatic device, process
    or means to access the Website for any purpose, including monitoring or
    copying any of the material on the Website."*
  - *"You must not access or use for any commercial purposes any part of the
    Website"*, and *"We provide this Website for use only by persons located
    in the United States."*
  - So: (a) an app fetching pages is an "automatic device": forbidden;
    (b) storing is forbidden; (c) bundling is forbidden. Other uses need a
    request to `legal@allbud.com`.
- **Privacy:** the policy (https://www.allbud.com/privacy-policy) says it
  stores *"your search activity, the pages you view ... your geographic
  location"* and may share *"which businesses you have visited"* with
  businesses on the site. Pages load Google Analytics, Google Maps, an ad
  network (`assets.mantisadnetwork.com`), ShareThis and reCAPTCHA.
- **Fit:** none. Rule out, apart from a link out.

### TerpHunter (verified 2026-10-15)
- A one-person project ("I built terphunter because I got tired of guessing",
  https://terphunter.com/about.html) covering Michigan, Ohio and Missouri
  dispensaries. A first attempt that day was refused by the proxy; the apex
  `terphunter.com` was allowed later and read. `www.terphunter.com` only
  redirects there.
- **Data** (https://terphunter.com/data.html): per product, 26 fields from
  dispensary menus and lab reports (COAs): 33 terpenes, 12 cannabinoids,
  harvest date, COA link, plus AI fields ("AI Predicted Effects", "AI Aroma
  Profile", "AI Intensity"). Per strain, 27+ fields, including "Family",
  "Chemotype", "Parent A", "Parent B", "Terpene Profile", "Dominant
  Terpene", "THC Range", "Indica %", "Medical Benefits". The home page
  claims *"5,200+ strain profiles"* and *"8,000+ products across MI & OH"*.
- **Provenance:** product terpenes and cannabinoids are measured lab data, but
  they belong to products on shelves, not to strains. The strain profiles are
  *"built from lab data, dispensary menus, multiple public databases, and AI
  synthesis"*, and the unnamed "public databases" carry the same provenance
  problem as the Kaggle scrapes. The terms add: *"Product data sourced from
  dispensaries and labs belongs to its respective owners."*
- **Access:** no public API. The search snippet's "$100/month Pro tier" is out
  of date: the pricing page (https://terphunter.com/pricing.html) lists
  everything free with an account, and **"Pro — Coming later: Full API
  access, Advanced analytics, Bulk data export"**. The site's own JSON API
  (`/api/strains/<name>`, `/api/strains/<name>/descendants`,
  `/api/products/search`) answers 401 `{"detail":"Not authenticated"}`
  without a login. The sitemap lists 22 pages and no strain pages. robots.txt
  allows everything except account pages.
- **Terms** (https://terphunter.com/terms.html, *"Last updated: March 27,
  2026"*):
  - Users agree not to *"Scrape, crawl, or bulk-download data from
    terphunter without permission"* or *"compete with terphunter by
    reselling or republishing our aggregated data"*.
  - *"Your use of terphunter does not grant you rights to redistribute our
    aggregated data or branding."*
  - *"You must be at least 21 years old"*. Accounts: *"Do not share your
    account."*
  - So: (a) live lookups would need each user's own TerpHunter login, sent
    from the app to an undocumented API: not offered, and close to scraping;
    (b) caching is not addressed; (c) bundling is redistribution: not
    granted. Nothing restricts non-commercial use as such.
- **Privacy** (https://terphunter.com/privacy.html, *"Last updated: March 18,
  2026"*): *"We do not sell your personal data. We do not run third-party
  display ads."* It logs *"page views, screen width, language"* first-party
  (`/api/stats/pageview`). The pages load only Google Fonts as a third party.
  A lookup would need an account (email), so every query would be tied to
  the user.
- **Fit:** the closest match to what Terpoir wants (parents, chemotype,
  terpene profile per strain), but there is no API to use and no licence to
  bundle, and the strain fields are partly AI-made from unnamed sources.
  Revisit when the Pro API ships: check whether its terms allow use from an
  app, caching, and a key per user, and ask where the strain data comes from.
- **Reply (2026-10-19 and 2026-10-20, by email):** TerpHunter's maintainer
  gave the Terpoir maintainer's own account Pro access, paid for out of
  pocket, and asked only that it not be abused, and for feedback.
  - **Access:** an MCP endpoint, `https://terphunter.com/mcp`, signed in
    with OAuth through a TerpHunter account (claude.ai custom connector or
    Claude Code), or a personal API key on request, sent as
    `Authorization: Bearer …` and *"don't share it"*. Its examples cover
    lineage and usual terpenes per strain, in-stock products by terpene,
    lab results per product, prices and state possession limits.
  - **Sources:** *"The data comes from dispensary menus and lab results and
    is updated nightly."* Nothing says which strain-level values are
    measured and which are inferred.
  - **Not answered:** permission to bundle (still redistribution under the
    terms), use from an open-source app without a shared key, and per-value
    provenance. The personal key cannot go in the app.
  - **Asked again (2026-10-21, Gmail draft for the maintainer to send):**
    the three questions, plus one design that would fit Terpoir's rules:
    each user signs in with their own TerpHunter account in the app (the
    OAuth flow the MCP endpoint already uses), and the phone queries
    TerpHunter directly: off by default, behind its own toggle and
    disclosure card, strain lookups only. Also asked whether such users
    would need Pro, whether the MCP endpoint may be called from an app, and
    whether a field marks measured versus inferred values.
  - **Meanwhile:** the maintainer evaluates data quality through the
    connector (strains they know well) and sends feedback. Any app use waits
    on the answers above and stays in 0.8.0 (online lookups).

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

Proxy returned `CONNECT tunnel failed, response 403` for these hosts on
2026-10-14:
`www.leafly.com`, `leafly.com`, `developer.leafly.com`, `consumer-api.leafly.com`,
`strainapi.evanbusse.com`, `otreeba.com`, `api.otreeba.com`, `www.kaggle.com`,
`www.wikidata.org`, `query.wikidata.org`, `huggingface.co`, `cannlytics.com`,
`kushy.net`, `opencannabis.info`, `www.cannabisreports.com`, `web.archive.org`,
`vdb.openthc.org`.
Reachable: `github.com`, `raw.githubusercontent.com` (git clone and raw files),
and search-engine snippets.

On 2026-10-15 `seedfinder.eu`, `en.seedfinder.eu`, `kannapedia.net`,
`www.allbud.com`, `terphunter.com` and `www.terphunter.com` were allowed and
read. `medicinalgenomics.com` and `www.medicinalgenomics.com` were refused at
first and allowed later that day.

Unverified as a result: all Leafly, Strain API, Otreeba, Kaggle and Wikidata
terms; the data licences for OpenTHC VDB and Cannlytics.
