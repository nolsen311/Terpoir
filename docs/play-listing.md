<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Google Play submission: copy-ready answers

Everything the Play Console asks for Terpoir's first Google Play release,
ready to paste. Written 2026-10-13 for the first Play release, version
0.2.0 (versionCode 2).

- **Sources:** the Play release build's merged manifest
  (`app/build/intermediates/merged_manifest/playRelease/`) and Google's own
  data disclosure pages, linked where used.
- **Maintainer decisions** (2026-10-13):
  - no background location in this release;
  - health declaration: Sleep Management only (2026-10-14, after Play
    rejected Medication and Treatment Management for a personal account);
  - developer name "Terpoir", contact `terpoirapp@gmail.com`, website
    `https://terpoir.app`;
  - no payments profile, so no in-app purchases.
- **Checked** against the Play Console Help Center on 2026-10-13: Data
  safety, App content, sign-in details, target audience, content ratings,
  health apps, foreground services, testing requirements, and the
  Inappropriate Content and Health Content and Services policies (linked
  in each section). The IARC questions themselves aren't published there:
  confirm them in the Console.
- **In the app** for the Health Content and Services and User Data
  policies:
  - the privacy policy as text: Privacy Center > "Privacy policy", from
    `site/privacy.html` bundled at build time;
  - the reminder to consult a healthcare professional, on Insights and in
    a session's Targeted symptoms section;
  - a disclosure before the camera and microphone permission prompts: what
    each is for, and that nothing leaves the phone.

Re-check this file whenever the Play build's dependencies or permissions
change.

## Store listing

**App name** (24/30):

```
Terpoir: Private Journal
```

**Short description** (79/80):

```
An encrypted journal for adults: note what you try and how it felt. No account.
```

**Full description** (2809/4000):

```
Terpoir is a private journal for adults who use cannabis where it is legal. Note what you tried, how much, and how it felt, then look back and see what actually works for you. It is a notebook, not a shop or a social network: Terpoir sells nothing, recommends nothing, and does not tell you to use more.

PRIVATE BY DESIGN
• No account, no sign-up, no Terpoir server. Your journal lives on your phone.
• Everything is encrypted on the device. It unlocks with your fingerprint, face or screen lock, and a recovery key that only you hold.
• No ads, no analytics, no advertising ID, no tracking. The developer never sees your journal.
• Kept out of phone backups, so your entries are never copied to a backup service.
• Every optional feature is a switch in the Privacy Center, with a plain card saying what it does, what leaves your phone, and how to undo it. The defaults are the most private settings.

WHAT YOU CAN RECORD
• Sessions: when, which products, how much, and the effects you felt, with an intensity for each.
• Products, brands and strains, with label details such as THC and CBD amounts and top terpenes.
• Notes in your own words, typed or spoken. Speech is turned into text by your phone's on-device recognizer; the audio is never kept.
• Saved places, with an approximate location if you want one.

QUICK TO USE
• Scan a product label, a receipt or a barcode, and the details fill themselves in. Text recognition runs on your phone.
• A quick-note tile and widget let you jot something down without opening the journal; the note is sealed until you unlock.

SEE YOUR OWN PATTERNS
• Insights are worked out on your phone from your own entries: which products tend to help you sleep or relax, and which ones you would rather skip.
• Mark an insight as wrong or hide it, and Terpoir remembers.

YOUR DATA STAYS YOURS
• Optional sync to a folder you choose, such as a Syncthing folder or your own cloud drive app. Only encrypted files are written there.
• Export everything at any time, as an encrypted file or a readable copy.
• Open source under the GPL. Anyone can read the code and check these promises.

OPTIONAL GOOGLE FEATURES
A few features use Google components. Each is off until you turn it on, and its card says what Google receives: Google Maps for picking a spot on a map, and Google's text recognition as an alternative scanner. Without them, Terpoir still works fully.

FOR ADULTS ONLY
Terpoir is for adults of legal age to use cannabis where they live. It asks you to confirm this on first launch.

NOT MEDICAL ADVICE
Terpoir is a personal self-tracking tool. It is not a medical device and does not diagnose, treat, cure, or prevent any medical condition. Consult a healthcare professional for medical advice, diagnosis, or treatment.

Privacy policy: https://terpoir.app/privacy
```

Notes on the copy:
- **Medical disclaimer (required):** the "NOT MEDICAL ADVICE" paragraph
  carries the sentence Google requires in the description of a health app
  that isn't a regulated medical device: "not a medical device and does not
  diagnose, treat, cure, or prevent any medical condition", plus the
  reminder to consult a healthcare professional
  ([Health Content and Services](https://support.google.com/googleplay/android-developer/answer/9878878)).
- **All-ages:** it names cannabis factually and says who the app is for.
  It has no imagery or wording that encourages use, and no ordering,
  delivery, dispensary, menu or price language. Google's marijuana policy
  bans facilitating sale "regardless of legality": an in-app cart, arranging
  delivery or pick-up, or selling THC products
  ([Inappropriate Content](https://support.google.com/googleplay/android-developer/answer/9878810);
  `docs/design-decisions.md`, "App store policy").
- **No background-location sentence:** background location isn't in this
  release, so the prominent-disclosure sentence isn't needed.
- **Graphics:** `android/release/store/icon-512.png` (app icon) and
  `android/release/store/feature-graphic-1024x500.png` (feature graphic).
  Phone screenshots are still to take, from the Play build with a sample
  journal: no real entries, nothing showing consumption.
- **Category:** Lifestyle. Health & Fitness invites medical-claim review
  that a self-tracking journal doesn't need.
- **Contact details:** email `terpoirapp@gmail.com`, website
  `https://terpoir.app`, privacy policy `https://terpoir.app/privacy`.
- **Developer name:** Terpoir.

## Privacy policy

`https://terpoir.app/privacy` (`site/privacy.html`).

## Ads

**Does your app contain ads?** No.

## Advertising ID

**Does your app use advertising ID?** No.

- **Manifest:** the Play release's merged manifest has no
  `com.google.android.gms.permission.AD_ID` permission.
- **Code:** no `AdvertisingIdClient` use.
- **Dependencies:** no ads or analytics SDK.

## Target audience and content

- **Target age groups:** 18 and over only.
- **Restrict Minor Access:** check it. Choosing 18 and over as the only
  group offers this box; with it, users Google determines to be minors
  can't search for, download or buy the app
  ([target audience](https://support.google.com/googleplay/android-developer/answer/9867159)). It isn't required for this
  kind of app
  ([Age-Restricted Content](https://support.google.com/googleplay/android-developer/answer/16302250)), but it is the store
  half of the age gating in `docs/design-decisions.md`.
- **Appeals to children:** No.
- **Store presence for children:** none. Not in Designed for Families, and
  not a Teacher Approved candidate.

## Sign-in details (formerly "App access")

There is no login, but the journal opens only with the device's screen
lock, which the reviewer sets up themselves. Google asks for clear
instructions whenever "your app requires users to set up their own
password (for example, a PIN)"
([sign-in details](https://support.google.com/googleplay/android-developer/answer/15748846)). Under **Sign-in details**,
choose **+ Add new instructions**, leave the username and password empty,
and paste this into **Any other instructions**:

```
No account or sign-in. Everything is on the device.

1. On first launch, confirm the age question ("I'm of legal age to use
   cannabis where I live").
2. Choose "Start a new journal". The journal is encrypted and unlocks with
   the device's biometric or screen lock, so the review device needs a
   screen lock (PIN, pattern or password) set before this step; without
   one the app asks you to set one.
3. Back up or skip the recovery key, then go through the Privacy Center
   tour. All features are then available.

Optional features (online map, Google text recognition, folder sync) are
off by default and are turned on in the Privacy Center (shield tab). No
test credentials are needed.
```

## Other App content declarations

- **COVID-19 contact tracing and status apps:** none of these.
- **News and Magazine apps:** not a news or magazine app.
- **Permissions declaration form:** not needed: no SMS, call log or other
  high-risk permission.

([App content](https://support.google.com/googleplay/android-developer/answer/9859455))

## Content rating (IARC questionnaire)

- **Email for rating notices:** `terpoirapp@gmail.com`.
- **Category:** All other app types (not a game).
- **Process** ([content ratings](https://support.google.com/googleplay/android-developer/answer/188189)): enter the email,
  pick the category, answer, review the calculated ratings, then submit.
  The table below is a guide: the live questionnaire's wording rules.

| Question | Answer | Why |
|---|---|---|
| Violence | No | None. |
| Sexuality, nudity | No | None. |
| Profanity or crude humour | No | None in app copy; user notes are private. |
| Fear or horror | No | None. |
| Controlled substances: references to or depictions of illegal drugs, alcohol or tobacco | Yes: references | It is a journal for logging legal cannabis use. |
| Encourages use of controlled substances | No | It records and reflects; it never prompts use (`docs/design-decisions.md`, "App store policy"). |
| Sale, purchase or trade of controlled substances | No | No ordering, delivery, menus or links. Price fields are the user's own records. |
| Gambling, simulated gambling | No | None. |
| Users can interact or exchange content | No | No accounts, chat or feed. Share cards go through the system share sheet, to whichever app the user picks. |
| Shares user's location with other users | No | Locations stay in the encrypted journal. |
| Digital purchases | No | None in this release. |
| Unrestricted internet or web browsing | No | No browser. The optional online map loads only map tiles. |

**Expected result:** drug references usually rate around Teen / PEGI 16.
That is below the design doc's "Mature". The 18+ target audience above
blocks children's accounts on top of whatever rating the questionnaire
gives, and the app's own age gate does the rest.

## Data safety

Answers follow from what leaves the phone. Nothing does unless the user
turns on an optional Google feature or agrees to a download, and then only
what Google's SDKs send to Google. Data that never leaves the device isn't
"collected", per Google's definition: the journal, locations, photos,
voice, and health-related entries. Google's data types and definitions
are on
[Declare your app's data use](https://developer.android.com/privacy-and-security/declare-data-use).

### Sources, per component of the Play release

| Component (merged manifest / dependency) | When it runs | What it sends to Google | Source |
|---|---|---|---|
| Maps SDK for Android (`play-services-maps`) | Only with the online map on and Google chosen | Request metadata (device OS version, name, model, brand; SDK version); crash stack traces and metrics; IP address; a pseudonymous Maps SDK identifier; map interaction events (pan, zoom) | [Maps SDK data disclosure](https://developers.google.com/maps/documentation/android-sdk/play-data-disclosure) |
| ML Kit text recognition (`play-services-mlkit-text-recognition`, with Google Data Transport) | Only with "Google text recognition" on | Device information; app package and version; a device identifier and per-installation identifiers; performance metrics; API configuration; input and output sizes; event types; error codes. Encrypted in transit, not transferred to third parties | [ML Kit data disclosure](https://developers.google.com/ml-kit/android-data-disclosure) |
| Play Feature Delivery and Play Asset Delivery | Only when the user agrees to download the open text-recognition engine | Device metadata and app version, to serve the right module and asset pack. Encrypted, not shared, deleted after a fixed period | [Feature Delivery](https://developer.android.com/guide/playcore/feature-delivery#data-safety), [Asset Delivery](https://developer.android.com/guide/playcore/asset-delivery#data-safety) |
| Play Age Signals (`age-signals`) | At first launch | Nothing: "No data is collected by the Play Age Signals client library." | [Play Age Signals data safety](https://developer.android.com/google/play/age-signals/overview#data-safety) |
| Google Play services location (`play-services-location`) | Not used: only "Notify me" geofencing calls it, which this release doesn't offer | Nothing | `docs/design-decisions.md` |
| Google Play services base, basement, tasks | Always (libraries) | Nothing: "do not collect any end-user data" | [Play services data disclosure](https://developers.google.com/android/guides/play-data-disclosure) |

### The form

**Does your app collect or share any of the required user data types?**
Yes: only through the optional Google features above.

**Is all of the user data collected by your app encrypted in transit?**
Yes, which Google allows only if it holds for every SDK. ML Kit states HTTPS. The Maps SDK and the Play Store talk to Google
over TLS.

**Which account creation methods does your app support?** None: the app
doesn't allow users to create an account.

**Do you provide a way for users to request that their data is deleted?**
No. Google allows Yes for a request mechanism, or for data deleted or
anonymized automatically within 90 days; neither is the developer's to
offer for Google's SDK data. There is no account and the developer holds no user data. The data
Google's SDKs collect is pseudonymous and governed by Google. The journal
is deleted by uninstalling or clearing the app's storage.

**Data types collected.** Collected, not shared. Google counts data an
SDK sends to its own provider as **collected**, and as **shared** when the
provider is a third party using it for itself, as Google does here ("to
improve Google services") rather than as the developer's service provider.
It isn't "sharing", though, when the transfer follows "a prominent in-app
disclosure and consent that meets the requirements described in our User
Data policy", or a user-initiated action the user expects
([Data safety](https://support.google.com/googleplay/android-developer/answer/10787469)). Each of these Google features is
off by default and is turned on from a Privacy Center card that says what
Google receives; the text-recognition download starts only from the user's
"Download" tap. ML Kit also states it doesn't transfer the data to third
parties.

| Data type | Collected | Processed ephemerally | Required or optional | Purposes | From |
|---|---|---|---|---|---|
| App info and performance: Crash logs | Yes | No | Optional | Analytics | Maps SDK |
| App info and performance: Diagnostics | Yes | No | Optional | Analytics, App functionality | Maps SDK, ML Kit, Play Feature and Asset Delivery |
| App info and performance: Other app performance data | Yes | No | Optional | Analytics | ML Kit (API configuration, sizes, event types, error codes) |
| Device or other IDs | Yes | No | Optional | Analytics | Maps SDK identifier; ML Kit device and per-installation identifiers |
| App activity: App interactions | Yes | No | Optional | Analytics | Maps SDK map interaction events |

Everything else is **not collected**:
- **Location:** spot locations stay in the encrypted journal. The Maps SDK
  receives the IP address; Google's Data safety guidance makes IP addresses
  location data only "where developers use IP addresses as a means to
  determine location", which Terpoir doesn't.
- **Personal info, financial info:** price fields stay on the phone.
- **Health and fitness, messages, photos and videos, audio** (on-device
  speech recognition), **files and docs, calendar, contacts, web browsing.**
- **App activity other than app interactions:** in-app search history,
  installed apps, other user-generated content, other actions.

**Security practices:** data encrypted in transit, Yes. No independent
security review yet.

Judgement calls, to revisit if Google's guidance changes:
- Play Delivery's device metadata is declared, although the Play Store
  process handles it.
- IP addresses aren't declared as location (Google's IP guidance above).
- Google's SDK data is collected, not shared, by the disclosure-and-consent
  exception. That holds only while each Google feature stays opt-in behind
  its Privacy Center card. If one ever turns on by default or without its
  card, declare its data types as shared too.
- **Optional** is allowed only if every user, on every device and in every
  region, can opt out. True here: every Google feature is a Privacy Center
  switch, off by default.

## Health apps declaration

**Category:** Sleep Management only.

Why not Medication and Treatment Management: it sits in the form's
**Medical** group, and Google says apps offering Medical health features
should come from an **organization** account
([account types](https://support.google.com/googleplay/android-developer/answer/13634885)). The first submission declared it
from a personal account and was rejected (2026-10-14). It also fit poorly,
since Terpoir has no schedules, reminders or adherence tracking. Sleep
Management is in the **Health and fitness** group, which a personal account
can publish.

Watch for: a reviewer may read the optional Targeted symptoms section
(pain, nausea, anxiety) as "Diseases and Conditions Management", also in
the Medical group. The listing, the symptoms card and the in-app reminder
present it as self-tracking, not condition management. If Google objects,
the choices are hiding Targeted symptoms in the Play build or an
organization account (a legal entity plus a free D-U-N-S number).

How the form works ([health apps declaration](https://support.google.com/googleplay/android-developer/answer/14738291)):
- Under **Policy > App content > Health apps**, tick the features, then
  **Next**.
- A second page asks for more information only when a feature needs it.
  The text below is for that page, and for any reviewer question.

Google's definitions ([health app categories](https://support.google.com/googleplay/android-developer/answer/13996367)):
- **Sleep Management:** "Apps dedicated to improving and tracking sleep
  patterns and quality."
- Camera, location and microphone are in scope of the health policy, which
  asks the app to "prominently disclose how user data will be used" for
  them, with affirmative consent. Location is asked from buttons in the
  spot editor. The scanner (`CameraScreen.kt`) and dictation
  (`Dictation.kt`) each explain their permission, with a Continue button,
  before Android's prompt.

```
Terpoir is a private, on-device journal for adults who use legal cannabis.
Users record each session (products, amounts, method, time) and how they
felt afterwards. It is a personal self-tracking journal, not a medical app.

Sleep Management: users can mark sleep as a session's intention, note a
"sleepy" effect, and record whether a session helped with sleep. Insights
compare products on those self-reported outcomes, from the user's own
entries only.

Terpoir is not a medical device and does not diagnose, treat, cure or
prevent any condition. It gives no medical advice or dosing
recommendations, and reminds users to consult a healthcare professional,
in the app and the listing. All health-related
entries stay encrypted on the device; there is no account or server, and
none of this data is collected by the developer or shared. It reads no
sensors and does not use Health Connect.
```

## Foreground service declaration

**Permission:** `FOREGROUND_SERVICE_DATA_SYNC`, from Play Asset Delivery.
Its `ExtractionForegroundService` has `foregroundServiceType="dataSync"`,
and Terpoir declares no foreground service of its own.

- **Task type:** Data sync.
- **Use case** (Play Console list,
  [foreground services](https://support.google.com/googleplay/android-developer/answer/13392821)): "Network transfer:
  Upload or download" (data fetch). Google's description includes "Data upload or
  download" and "Local file processing"
  ([foreground service types](https://developer.android.com/develop/background-work/services/fgs/service-types)).
- **Feature:**
  ```
  On-demand download of the open text-recognition model (Play Asset
  Delivery, asset pack "ocr_model"). When the user chooses to scan a receipt
  or label and agrees to download text recognition, Google Play downloads
  the model and Play Asset Delivery's extraction service copies it into the
  app's storage as a foreground service with a progress notification. It
  runs only after that explicit tap, once per install, and stops when the
  model is in place.
  ```
- **User impact if deferred or interrupted:**
  ```
  The user is waiting on the scan screen for the model. If the copy were
  deferred or stopped, scanning would stay unavailable and the user would
  have to type the receipt or label by hand; a partial copy would have to be
  downloaded again.
  ```
- **Video (about 30 seconds):** the form takes a link to it, for example
  an unlisted YouTube video. It must show the steps that start the
  service.
  - **0:00–0:05:** open Terpoir, unlock, and tap "Scan receipt" from a new
    purchase.
  - **0:05–0:10:** the "Text recognition needs a download" card; tap
    "Download".
  - **0:10–0:22:** Google Play's download progress, then the system
    notification for the asset pack being copied (pull down the shade to
    show it).
  - **0:22–0:30:** the notification disappears on its own, and the camera
    opens to scan the receipt.

## Location declaration

Not needed for this release: the Play build doesn't request
`ACCESS_BACKGROUND_LOCATION`. "Show me" and spot locations use location
only while the app is open. If "Notify me" comes to Play later, it needs
the background-location declaration and a video
(`docs/design-decisions.md`, "Notify me").

## Release

- **Track:** internal testing first, as a draft
  (`android/scripts/play-upload.sh`, `docs/releasing.md`).
- **Release notes** (en-US, the "What's new" box, 500 characters at most).
  `play-upload.sh` doesn't send them: paste them into the release in the
  Play Console before rolling it out.
  - **Google Maps** first appears in 0.2.1: 0.2.0 was built before the
    release Maps key existed (Terpoir-infra applied it on 2026-10-14), and
    a Play build without the key doesn't offer Google Maps.
  - **0.5.2** (430 characters):
    ```
    <en-US>
    Fixes "secure key storage did not respond" when unlocking, which could keep coming back until the phone restarted. The unlock screen is now all Terpoir green and shows the app's version. Turning on folder sync shows its folder and Sync buttons right away. On Products, the New button floats at the bottom and products are listed by strain; a product no longer needs a name of its own. In the journal, New entry is the main button.
    </en-US>
    ```
  - **0.5.1** (404 characters):
    ```
    <en-US>
    Insights now group strains by family: a parent strain and the strains bred from it, so you can see how a whole family works for you, not just one strain at a time. Shop insights also compare strain families and a product's main terpene, so a shop's difference shows up even when you never bought the same strain in two places. Families use parent strains, which show at the Detailed step in What I track.
    </en-US>
    ```
  - **0.5.0** (472 characters):
    ```
    <en-US>
    New ways to log a session: hold the bud and flick it to show how it's hitting, then fill in the rest of the timeline later. The date, spot, reminder and notes are now quick icons, and closing asks before throwing anything away. Products can record CBN, CBG, THCa and CBDV from the label. Settings is now one tab, with a theme choice (phone, light or dark) and clearer cards in dark mode. Folder sync shows when it's working. Fixes a crash when the app locks while loading.
    </en-US>
    ```
  - **0.4.0** (411 characters):
    ```
    <en-US>
    Richer strains. Choose how much detail you keep in What I track: Simple, Terpenes or Detailed. Terpenes lets you pick the ones a strain is known for (most have one to three). Detailed adds how strong each one is, parent strains (from your journal, or just type a name) and other cannabinoids like CBG and CBN. When a product's label lists no terpenes, insights use its strain's main terpene instead, and say so.
    </en-US>
    ```
  - **0.2.2** (353 characters):
    ```
    <en-US>
    A clearer Privacy Center: each area opens on its own page, and anything you've turned on shows what it shares. Exports and folder sync now check every file was saved, and say so if your storage app didn't keep it. If something goes wrong, you can send a report from your email app; nothing from your journal is in it. Notifications use the Terpoir icon.
    </en-US>
    ```
  - **0.2.1** (336 characters):
    ```
    <en-US>
    New: Google Maps for picking a spot's location. It's optional and off until you choose it, and it tells you what Google sees first. Unlocking is quicker: the fingerprint or face prompt now appears as soon as you open the app. If a journal was made by a newer version of Terpoir, the app now says to update instead of failing to open it.
    </en-US>
    ```
  - **0.2.0** (296 characters):
    ```
    <en-US>
    First release on Google Play. Terpoir is a private, encrypted journal: no account, no ads, no tracking. Already have a journal? Restore it from your recovery key or a sync folder. The privacy policy is in the Privacy Center, and every optional feature explains what it does before you turn it on.
    </en-US>
    ```
- **Closed testing:** personal developer accounts created after
  13 November 2023 must run a closed test with at least 12 testers, opted
  in continuously for 14 days, before applying for production access. The
  application asks for a summary of their feedback, so keep a record of
  it. The rule applies only to those personal accounts
  ([testing requirements](https://support.google.com/googleplay/android-developer/answer/14151465)).
  - **Testers (maintainer, 2026-10-14):** the alpha track's testers are the
    Google Group [terpoir-testers](https://groups.google.com/g/terpoir-testers),
    which anyone can join, beside the original email list. The website's
    "Help test it" box sends people to the group first, so adding a tester
    takes no step from the maintainer. The group's members are visible only
    to its managers, and it carries no posts. Play's API can attach groups
    to a track but not single addresses, so the email list can't be
    scripted.

