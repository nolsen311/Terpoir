<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Simpler receipt scanning: options

For the maintainer's request of 2026-10-15: "We should work on some options
for the receipt scanning to make it much simpler and intuitive for text
recognition to populate the right parts of a new entry." Written 2026-10-15
from the code as of version 0.5.2. Nothing here is decided. The maintainer
picks an option (or a mix), and the choice then goes into
`docs/design-decisions.md`, "Rapid product entry".

Every option keeps the rules that already apply: text recognition and parsing
run on the device; parsing and matching live in the Rust core (`terpoir-scan`,
the `Journal` matchers), and Android only shows the results; the receipt photo
is never written to storage; personal and payment lines are never offered on
their own; nothing goes into a data point the user has hidden in "What I
track".

## Today

**How the user gets there:** "Scan receipt" at the top of the Products list,
"log a purchase" from "Show me", or a "Quick log?" purchase prompt. On Play,
the first scan offers to download Tesseract.

**What the user sees and does** (`ReceiptScreen.kt`):

1. Camera: "Fit the whole receipt in the frame." Take the photo.
2. "Reading…"
3. One long review screen, top to bottom:
   - "Check each line…" and "Tap a line on the photo…" hints;
   - the photo with a box on every printed row: tap a row to add or remove
     it as an item, press and hold a row to make it the shop;
   - Shop: a dropdown (Don't record / each saved spot / "New Re-up Spot: …"),
     plus a name field for a new spot;
   - Date; Currency (a three-letter code typed into a text field);
   - Items: one card per line, each with a checkbox, an Item text field, a
     Price text field and a Product dropdown ("New product", then every
     product in the journal, A to Z);
   - "Add item".
4. Save: one Purchase. Each item linked to nothing becomes a **new product
   named with the printed text**, and nothing else filled in.

**What the text becomes** (`terpoir-scan/src/receipt.rs`): the shop (first
header line that looks like a name), the date, the total, and per item a name,
a quantity and a price. Product matching (`Journal::find_products`) is an
exact match on the normalised name. Shop matching (`Journal::match_spot`)
takes an exact or whole-word match.

**Friction:**

- **The right parts aren't filled.** "BD 3.5G HYB" becomes a product called
  "BD 3.5G HYB" with no brand, strain, form, size or potency. To get a usable
  product the user opens each one later, renames it, and fills in or scans
  the rest. That is the main complaint.
- **Matching seldom works.** Receipts abbreviate, so the exact-name match
  almost always misses, and the user scrolls a dropdown of every product.
  Learned receipt names and OCR-tolerant suggestions are decided ("Scan recognition from the journal") but not built.
- **Some parsed data is lost.** The parser reads quantities ("2 x Gummies"),
  but `PurchaseLine` has nowhere to keep them, so they're dropped. Size
  ("3.5g", "100mg") has no field on a product or a purchase line, so it stays
  in the name. The printed total is read but never compared with the items, so
  a missed line goes unnoticed.
- **Hard to find, hard to see.** Press-and-hold for the shop can't be
  discovered. The photo and the item cards are far apart on one scroll, so a
  tap on the photo changes something off screen. Each card has three fields
  plus a dropdown, so a five-line receipt means about 20 controls.
- **Label and receipt don't meet.** The label scan is only in the product
  editor, one product at a time. It fills THC/CBD, terpenes, batch, package
  date and package tags. The design doc says it also pre-fills name and
  brand, but the code doesn't.
- **No way on to a session.** A purchase can't start a "New entry", even
  though a product's form already gives a default method
  (`Form::default_method`).

## Building blocks every option needs

These are shared, so they are listed once. Each option says which it uses.

- **Item breakdown (core).** A function such as
  `terpoir_scan::describe_item(text, vocabulary)` that splits a line's text
  into parts, each with a confidence: size (`3.5g`, `1/8`, `eighth`, `100mg`,
  `1g`), form words (`cart`, `vape`, `pre-roll`/`PR`, `gummies`, `flower`,
  `live resin`, `wax`, `tincture`…), strain type words (`ind`, `sat`, `hyb`),
  quantity, and whatever is left as the name. The vocabulary is built in
  (like the terpene list) plus the journal's own brands, strains and
  products. This is the "growing over time" part of "Scan recognition from
  the journal", started early.
- **Ranked suggestions (core).** The OCR-tolerant ranking already decided:
  it ranks products, brands and strains against the text and boosts the
  shop's usual products.
- **Data model choices (maintainer).** Before anything can be stored:
  - where size goes. Suggestion: on `PurchaseLine` (the same product can be
    bought as 3.5 g and 7 g), with an optional default size on `Product`;
  - whether `PurchaseLine` gets a quantity.

  Both are synced record changes that need a compatibility fixture and must
  appear in the plaintext export.

---

## Option A: Scan and review cards

One photo. Every item the receipt shows is already broken down into a product
card with chips. The user checks the cards and taps chips to fix them; the
photo is there only to look at.

**The user's steps**

1. Take the photo; "Reading…".
2. A short header: shop chip ("Green Leaf ✓ matched", or "New spot: GREEN
   LEAF"), date chip, and "3 items · $116 of $127.65 total" (see the
   mismatch note below).
3. One card per item: the printed text small and grey on top, then chips:
   **Product** ("Acme Blue Dream ✓" or "New"), **Brand**, **Strain**,
   **Form**, **Size**, **Qty**, **Price**. A chip with a "?" is a guess.
   Tap a chip to see the next suggestions, pick another, or type. Swipe a
   card away to drop it.
4. "Missing a line?" opens the photo with today's tap-to-add rows.
5. Save. New products are created with name, brand, strain and form filled;
   price, size and quantity go on the purchase line.

**Fields and confidence**

| Field | Source | Confidence |
|---|---|---|
| Price, date, total | pattern parser (today) | high |
| Quantity | pattern parser (today) | high, when printed |
| Size | item breakdown | high for `3.5g`/`100mg`, medium for `1/8`, `eighth` |
| Form | item breakdown (form words) | medium; "?" chip |
| Brand, strain | journal vocabulary, then leftover words | high when already in the journal, otherwise a "New?" guess |
| Product | learned alias, then ranking | automatic only on an exact alias, otherwise "Looks like…?" |
| THC/CBD, terpenes | not on receipts | none (a "Scan label" chip can fill them, see C) |

**New work**

- Core: item breakdown; ranked suggestions; a total check (sum of the kept
  lines against the printed total); size and quantity on the purchase line.
- Android: replace the item text-field cards with chip cards; shop and date
  as header chips; a searchable product picker instead of the A to Z
  dropdown; the photo behind "Missing a line?".

**Effort:** M–L (most of it is the core breakdown and the data-model change).

**Risks**

- OCR errors in names make the chips guess wrong. The "?" marking and
  one-tap fixes keep that cheap, and nothing below an exact alias links on
  its own.
- Long receipts (10+ lines) give a long card list. Cards are compact (one
  line of chips), and the total check flags a missed or extra line.
- Receipt-only text has no potency, so potency stays empty unless the user
  also scans the label.

**Privacy:** nothing new leaves the encrypted store. The vocabulary is read
from the unlocked journal in memory. No new toggle. The photo is kept in
memory exactly as today.

```
┌ Scan receipt ─────────────────── Save ┐
│ [Green Leaf ✓]  [Oct 11 ▾]            │
│ 3 items · $116.00 of $127.65 ⚠        │
├───────────────────────────────────────┤
│ "BD 3.5G HYB              35.00"      │
│ [Acme Blue Dream ✓] [Flower] [3.5 g]  │
│ [Hybrid] [$35.00]                     │
├───────────────────────────────────────┤
│ "2 x MANGO GUMM 100MG     36.00"      │
│ [New product] [Brand ?] [Edible ?]    │
│ [100 mg] [×2] [$36.00]                │
├───────────────────────────────────────┤
│ Missing a line? Show the photo        │
└───────────────────────────────────────┘
```

---

## Option B: Tap to assign on the photo

The photo is the screen. Every recognised word or phrase has a box. The user
picks a field at the bottom, then taps the boxes that belong to it. The same
screen works for receipts and labels.

**The user's steps**

1. Take the photo; "Reading…".
2. The photo fills the screen (pinch to zoom). The parser's guesses are
   already coloured: green for item names, blue for prices, purple for the
   shop, orange for the date.
3. A field bar along the bottom: **Item · Price · Brand · Strain · Size ·
   Shop · Date · Skip**. Tap a field, then tap boxes on the photo. The field
   stays selected, so "Price" then four taps sets four prices. A box tapped
   into "Item" starts a new item, and the boxes on the same row after it join
   that item.
4. A slide-up sheet shows the entry being built ("Item 1: Blue Dream ·
   3.5 g · $35"), so the effect of each tap shows at once.
5. Save.

**Fields and confidence:** whatever the user taps is certain. The
pre-colouring has the parser's confidence (today's level). This option
doesn't make guesses better; it makes fixing them direct.

**New work**

- Core: word-level boxes. `group_rows` already takes positioned pieces;
  rows would keep their pieces instead of only the joined text.
  `parse_field(kind, text)` reads a tapped box as a price, date, size or
  potency.
- Android: a zoomable photo with per-word hit targets, the field bar, the
  live sheet, and an accessible list alternative (TalkBack can't tap boxes on
  a picture).

**Effort:** M (mostly Android).

**Risks**

- Small print means small targets. It needs zoom, and the slack around rows
  that today's view has.
- Many taps on a long receipt. It's best as a correction mode, not the main
  path.
- Engines differ: Tesseract with `PSM_SINGLE_COLUMN` gives row pieces, and
  word boxes may need another call per row. Worth testing on the foss
  `tessdata_fast` model before choosing.
- Accessibility needs the list alternative; without it this option fails.

**Privacy:** the in-memory photo is central for longer, but it still lives
only while the screen is open and is never written to storage. Personal
lines can be tapped into a field deliberately, as today.

```
┌ Scan receipt ─────────────────── Save ┐
│   GREEN LEAF DISPENSARY  ← shop       │
│   10/11/2026 2:02 PM     ← date       │
│  [BD 3.5G HYB]..........[35.00]       │
│  [2 x MANGO GUMM 100MG]..[36.00]      │
│   Subtotal               111.00       │
├───────────────────────────────────────┤
│ Item 1: BD 3.5G HYB · $35   ▲ sheet   │
├───────────────────────────────────────┤
│ Item Price Brand Strain Size Shop Skip│
└───────────────────────────────────────┘
```

---

## Option C: Guided capture, label first

Compliance labels carry the product facts (brand, strain, form, potency,
terpenes, batch, package tag) in a fixed, readable layout. Receipts carry
the money. So each scan reads what it's good at: "Log a purchase" scans each
package's label (or code), then one receipt photo for price, shop and date.

**The user's steps**

1. "Log a purchase" → "Scan each package" (camera, label mode). One label
   per photo. A package tag or barcode already in the journal finds the
   product at once ("Acme Blue Dream, again"). Otherwise the label fills a
   new product card.
2. After each label: "Next package" or "Done with packages". The tray at the
   bottom shows one thumbnail chip per product.
3. "Scan the receipt" (or "Skip"). The core lines the receipt rows up with
   the scanned products by name, brand and size, and fills each price. The
   shop and date go in the header.
4. Review: product cards with price chips. An unmatched receipt line shows
   as "Which product is $36.00 for?" with the products as choices.
5. Save. Products come out complete (with potency and terpenes), and so does
   the purchase.

**Fields and confidence**

| Field | Source | Confidence |
|---|---|---|
| Product (re-buy) | package tag or barcode | exact |
| THC/CBD, terpenes, batch, package date | label parser (today) | high |
| Brand, strain, form, name | label parser, extended | medium to high (labels print these on their own lines) |
| Size | label net weight (`Net Wt 3.5g`) | high |
| Price | receipt row matched to a product | medium; unmatched rows asked for |
| Shop, date | receipt (today) | as today |

**New work**

- Core: extend `parse_label` to brand, strain, form, name and net weight
  (using the journal vocabulary); `match_receipt_to_products(receipt,
  products)` lines receipt rows up with products.
- Android: a multi-capture flow with a tray, the product picker reused, the
  label scan available outside the product editor.

**Effort:** L (two parsers grow, plus a new flow).

**Risks**

- More photos: one per package plus the receipt. Re-buys are fast (a code
  scan, no photo), but a first haul of five products means six photos.
- Labels on round jars and small edibles are curved and tiny, so OCR is
  weaker there than on flat receipts; the code scan helps.
- Labels differ by state and country; the label parser has to stay
  conservative.
- Lining up receipt rows fails when the receipt prints only a SKU. The
  question card catches that.

**Privacy:** label photos are recycled at once as today (or kept as the
product picture, EXIF-stripped, where the user already chose that). Package
tags are precise identifiers and stay in the encrypted product, as today. No
online lookup.

```
┌ Log a purchase ───────────────────────┐
│  ┌───────────── viewfinder ────────┐  │
│  │   ACME · BLUE DREAM · 3.5g      │  │
│  │   THC 24.1%  CBD <0.1%          │  │
│  └─────────────────────────────────┘  │
│  Scanned: [Blue Dream] [Mango Gummies]│
│  [Next package]  [Done → scan receipt]│
└───────────────────────────────────────┘
```

---

## Option D: Learn from corrections

Whatever the user fixes once is remembered, so the next receipt from the same
shop comes out right. This is not a screen of its own: it sits behind A, B
or today's screen and makes each one quieter over time.

**The user's steps**

1. First receipt from a shop: as in whichever screen is used. The user fixes
   "BD 3.5G HYB" to Acme Blue Dream, flower, 3.5 g, and unticks "CANNABIS
   EXCISE 15%".
2. A one-line note on save: "Terpoir will recognise these next time at
   Green Leaf." Undo is there.
3. Next receipt from Green Leaf: the shop is matched from the header, "BD
   3.5G HYB" is already Acme Blue Dream ("✓ from last time"), "BD" alone
   reads as Blue Dream, and the excise line isn't offered.
4. A "What Terpoir learned" list on the spot (and the product) to review or
   forget entries.

**What it learns, per shop and encrypted, with the spot or product:**

- exact line aliases → product (already decided as "Learned receipt names");
- abbreviations → strain, brand or form (`BD` → Blue Dream, `PR` → pre-roll,
  `HYB` → hybrid), seen at least twice before they apply on their own;
- lines to skip (normalised, with digits removed so no number is stored);
- the header line that names the shop, and the date order (day/month or
  month/day).

**Fields and confidence:** exact aliases link on their own (as decided).
Learned abbreviations fill chips marked "from last time", which one tap
changes. It gets better with each receipt and starts at today's level.

**New work**

- Core: a per-spot "receipt vocabulary" (on the Spot record, or as a small
  record of its own that syncs with it), applied before the item breakdown;
  normalisation versioning as already noted for aliases.
- Android: the "remembered" note and undo; the review/forget list.

**Effort:** M in the core, S on Android, and it needs a correction screen (A
or B) to learn from.

**Risks**

- A wrong correction gets repeated. That's why there's the visible "from
  last time" chip, undo on save, and a forget list.
- Shops change their till software. Learned entries that stop matching just
  stop applying.
- The first receipt from a new shop gets no benefit.

**Privacy:** learned entries are journal data: encrypted, synced, in the
plaintext export, never on share cards, and deleted with the spot or
product. They are never given to the OCR engine (decided: no vocabulary
files on disk). Skip rules keep no digits, so no card or member number is
ever stored. No new toggle.

```
┌ Scan receipt · Green Leaf ✓ ──── Save ┐
│ "BD 3.5G HYB              35.00"      │
│ [Acme Blue Dream ✓ last time] [3.5 g] │
│ "PR JACK H 1G             12.00"      │
│ [Pre-roll ✓] [Jack Herer ✓] [1 g]     │
│ Skipped: 1 line you skip here  (show) │
└───────────────────────────────────────┘
  Saved. Terpoir will recognise these
  next time at Green Leaf.      [Undo]
```

---

## Add-on for any option: on to a new entry

After saving a purchase: "Start an entry with Blue Dream?" opens "New entry"
(a session) with the product line filled in: the product, the method from its
form (`Form::default_method`), and the purchase line linked (`PurchaseRef`).
The core already holds every piece, so this is S on Android. It answers "a
new entry" in the maintainer's request most directly.

## Comparison

| | A: cards | B: tap on photo | C: label first | D: learning |
|---|---|---|---|---|
| Fills brand, strain, form, size | guessed, then confirmed | by hand | from the label | from last time |
| Fills potency, terpenes | no | if tapped on a label | yes | no |
| Taps for a 3-item receipt | ~3–6 | ~10–15 | 4 photos + ~3 | ~0–3 by the 2nd visit |
| New core work | breakdown, ranking | word boxes, field parse | label fields, row matching | per-shop vocabulary |
| Effort | M–L | M | L | M (+ A or B) |

## Recommendation

**A with D behind it**, B's photo kept as the "Missing a line?" fallback, and
C's label scan as a per-card chip instead of a separate flow.

A gives the biggest step from today: the receipt fills real product fields,
and fixes are one tap. D is mostly work already decided ("Scan recognition
from the journal"), and it's what makes the second visit to a shop nearly
tap-free. B is the right correction tool but too slow as the main path. C
gives the most complete products, but it asks for a photo per package, so
it fits better as "Scan label" on a card that wants potency.

**A sensible first slice:**

1. Core: `describe_item` (size, form, strain type, quantity, leftover name)
   with the built-in vocabulary, plus the journal's brands and strains;
   tests from real receipt text.
2. Maintainer decision on size and quantity on `PurchaseLine` (and
   default size on `Product`), with the compatibility fixture.
3. Android: chip cards, the header with shop and date, a searchable product
   picker, the photo behind "Missing a line?". New products saved with name,
   brand, strain and form filled.
4. The "Start an entry with…" hand-off.

Next, learned aliases and abbreviations (D, and the "Scan recognition from
the journal" roadmap item), then "Scan label" on a card (C's parser work).

**Decisions for the maintainer:** which option(s); where size and quantity
live; whether learned abbreviations are per shop only or also used for every
shop once confirmed; whether the hand-off to "New entry" is offered every
time or only from "Quick log?".
