# terpoir.app

The project website: a single static page with no analytics, cookies,
third-party scripts, fonts or external requests. `_headers` sets a strict
Content-Security-Policy and other response headers.

Artist slots are marked in `index.html`; search for `ARTIST:`. Everything
visual is plain type and CSS until there is human-made artwork to drop in.

## Deploying on Cloudflare (free)

Static asset requests are free and unlimited on Cloudflare's free plan, so
this costs nothing.

1. In the Cloudflare dashboard: **Workers & Pages → Create → Pages →
   Connect to Git**, and pick `nolsen311/Terpoir`. Cloudflare's GitHub app
   only needs access to this repository.
2. Build settings: framework preset **None**, build command **empty**,
   build output directory **`site`**. Production branch: `main`. Under
   **Build watch paths**, include only `site/*` so app changes don't
   trigger site builds.
3. After the first deploy, **Custom domains → Set up a custom domain →
   `terpoir.app`**. The zone is already on Cloudflare, so it adds the record;
   the existing CAA records already allow Cloudflare's certificate
   authorities.

Every merge to `main` then redeploys the site. Pull requests get preview URLs.

Keep **Web Analytics off** for the project and the zone: it injects a
third-party beacon script, which the CSP would block and the page promises
not to have.
