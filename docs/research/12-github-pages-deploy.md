# Research 12 — Auto-deploy WASM demo to GitHub Pages (v2.1.0)

Goal: auto-deploy the `web/` demo (loads `web/pkg/tte_wasm.js` + `tte_wasm_bg.wasm`)
to GitHub Pages on merge to `main`, via the official Actions Pages flow.

Project facts (from repo):
- `wasm-bindgen` pinned at **0.2.123** (Cargo.lock).
- Build = `cargo build -p tte-wasm --profile release-wasm --target wasm32-unknown-unknown`
  → `wasm-bindgen --target web --no-typescript` → optional `wasm-opt -Oz`. Script: `web/build.sh`.
- `web/index.html` loads `<script type="module" src="./app.js">`; `web/app.js` does
  `import init, { Renderer } from "./pkg/tte_wasm.js"` and `import { GridRenderer } from "./renderer.js"`.
  **All paths are relative (`./...`)** → already subpath-safe for `user.github.io/REPO/`. GOOD.
- Existing CI (`.github/workflows/ci.yml`) already has a `wasm` job using
  `dtolnay/rust-toolchain` (targets wasm32), `Swatinem/rust-cache@v2`,
  `taiki-e/install-action@v2` (tool: wasm-bindgen), and runs `./web/build.sh`.
- Repo: `github.com/emalenchek/3d-rendering-engine` (public personal repo).

---

## Q1 — Modern Pages deploy flow (configure/upload/deploy) — confidence HIGH

- Official flow = three actions, NOT the legacy `gh-pages` branch / peaceiris:
  `actions/configure-pages` → `actions/upload-pages-artifact` → `actions/deploy-pages`.
- Required by `deploy-pages`: job permissions `pages: write` **and** `id-token: write`
  (OIDC), and the job must target the `github-pages` environment.
- `concurrency: group: "pages", cancel-in-progress: false` — serialize deploys so an
  in-flight deploy isn't cancelled (recommended in official docs).
- Current action versions (June 2026): `actions/deploy-pages@v4`,
  `actions/upload-pages-artifact@v5` (v5.0.0, Apr 2026), `actions/configure-pages@v5`,
  `actions/checkout@v4`.
- `upload-pages-artifact` default source path is `_site/`; override with `path:` (we use `web/`).
  Has `include-hidden-files` (default false); excludes `.git`/`.github` always.
- Sources:
  - https://github.com/actions/deploy-pages
  - https://github.com/actions/upload-pages-artifact
  - https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages

## Q2 — Building wasm in the deploy job — confidence HIGH

- Reuse the existing CI recipe verbatim (it already builds the artifact):
  - `dtolnay/rust-toolchain@stable` with `targets: wasm32-unknown-unknown`.
  - `Swatinem/rust-cache@v2` for build caching.
  - `taiki-e/install-action@v2` with `tool: wasm-bindgen` — installs prebuilt
    `wasm-bindgen-cli`. taiki-e resolves the version from the project's `Cargo.lock`
    when run inside the repo, so it matches the pinned 0.2.123 (avoids the classic
    "rust-bindgen and CLI versions don't match" failure). Pin explicitly as
    `tool: wasm-bindgen@0.2.123` for determinism.
  - Run `./web/build.sh` directly — single source of truth, also enforces the NFR-7
    size budget. binaryen/wasm-opt is optional; the script no-ops if absent. To get
    the size pass in CI, add `tool: wasm-opt` (binaryen) to the install-action list.
- Sources:
  - https://github.com/taiki-e/install-action
  - https://github.com/jetli/wasm-bindgen-action (alternative; jetli downloads a release binary)
  - existing `.github/workflows/ci.yml` wasm job

## Q3 — Pages serving specifics for WASM — confidence HIGH

- **MIME:** GitHub Pages production *does* serve `.wasm` as `application/wasm` (mime-db
  includes it). The recurring "Response has unsupported MIME type 'text/html'" reports
  are almost always a **404**: the requested path doesn't exist, so Pages returns the
  HTML 404 page (text/html). Root causes: wrong relative path, or Jekyll stripping a
  dir. Fix = correct paths + `.nojekyll`. Note: `jekyll serve` *locally* serves wrong
  MIME — that is a local-only artifact, not production.
- **`.nojekyll` REQUIRED:** add an empty `.nojekyll` at the **published root** (i.e. in
  `web/`, since we upload `web/` as the artifact). Without it Jekyll runs and strips any
  file/dir beginning with `_` and adds latency. The repo currently has **no `.nojekyll`**
  — must add `web/.nojekyll`. (`pkg/` itself has no underscore, but keep `.nojekyll` as
  the canonical "serve verbatim" switch.)
- **Base path / subpath:** project site is served at `https://emalenchek.github.io/3D-Rendering-Engine/`
  (the repo-name path segment is **case-sensitive** — it must match `3D-Rendering-Engine` exactly).
  All asset references MUST be relative (no leading-slash absolute paths). VERIFIED OK:
  `index.html` → `./app.js`; `app.js` → `./pkg/tte_wasm.js`, `./renderer.js`. The
  wasm-bindgen `--target web` glue loads `tte_wasm_bg.wasm` relative to the JS module via
  `new URL('tte_wasm_bg.wasm', import.meta.url)`, so `pkg/` resolves correctly under the
  subpath with no `--base`/`init(path)` override needed.
- `app.js` uses `await init()` (default). wasm-bindgen's generated init uses
  `instantiateStreaming` and **auto-falls back** to non-streaming if MIME is off, so even
  a worst-case MIME glitch degrades gracefully (slower, not broken).
- Sources:
  - https://github.com/orgs/community/discussions/22863 (Pages serves application/wasm; text/html = 404 fallback)
  - https://docs.github.com/en/pages/setting-up-a-github-pages-site-with-jekyll/about-github-pages-and-jekyll (underscore files stripped without .nojekyll)
  - https://rustwasm.github.io/book/reference/deploying-to-production.html
  - https://github.com/github/pages-gem/issues/695 (local jekyll-serve MIME is the wrong-MIME source, not prod)

## Q4 — First-time setup — confidence HIGH

- Repo **Settings → Pages → Build and deployment → Source = "GitHub Actions"** (one-time,
  manual). This switches off the legacy branch-based publish.
- Enablement: `actions/configure-pages` has an `enablement: true` input that can turn Pages
  on programmatically, but it needs the right token scope and is unreliable on first run —
  recommend setting the Source in Settings once by hand, then the workflow runs cleanly.
  Without the source set, the first `deploy-pages` run errors with "Pages site not found".
- Public personal repo (this case): Pages is free, no plan constraints. Private-repo Pages
  requires GitHub Pro/Team/Enterprise; not applicable here.
- Sources:
  - https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site
  - https://github.com/actions/configure-pages

## Q5 — Making the demo testable in CI — confidence MEDIUM-HIGH

- Best fit: a small **Playwright smoke test** that serves the built `web/` with
  `python3 -m http.server` and asserts the demo boots. `python3 -m http.server` serves
  `.wasm` as `application/wasm` (matches prod), so it's a faithful local stand-in.
- Minimal recipe in a CI job (after `./web/build.sh`):
  `npx --yes playwright@latest install --with-deps chromium`, start the server in the
  background, then a ~30-line Node script via `page.goto('http://localhost:8000/')` that:
  (1) collects `console` + `pageerror` events and fails on any error, (2) waits for
  `#status` text to match `\d+×\d+ · \d+ FPS` (proves WASM init + render loop ran),
  (3) optionally checks the `#screen` canvas is non-blank. Run headless (default).
- Lightweight alternative: a headless-Chrome (puppeteer / `chrome --headless --dump-dom`)
  script, but Playwright's auto-wait + console capture is more reliable for the same LOC.
- Gate placement: run this as a **PR + push job in ci.yml** (catches breakage before
  merge); it does not need to block the deploy job (deploy only runs on push to main).
- Sources:
  - https://playwright.dev/docs/ci-intro
  - https://github.com/microsoft/playwright-github-action
  - https://playwright.dev/docs/ci

## Q6 — Per-PR preview deployments — confidence HIGH

- Verdict: **skip for now / impractical with the official artifact flow.** The official
  `upload-pages-artifact`/`deploy-pages` model deploys exactly ONE artifact to the single
  Pages site — it has no native sub-path/preview multiplexing.
- The popular `rossjrw/pr-preview-action` works by committing each PR's build into a
  `pr-preview/<n>/` subdir on the **legacy `gh-pages` branch** (Read/write workflow perms).
  That means running the *branch-based* publish model in parallel with (or instead of) the
  Actions source — a meaningful complication and a different security posture.
- Recommendation: rely on the CI Playwright smoke test (Q5) for per-PR confidence; if real
  shareable previews are later wanted, adopt `rossjrw/pr-preview-action` on a dedicated
  `gh-pages` branch, accepting the dual-model cost. Not worth it for v2.1.0.
- Sources:
  - https://github.com/rossjrw/pr-preview-action
  - https://github.com/rossjrw/pr-preview-action/blob/main/README.md

## Q7 — Caching / versioning gotchas — confidence MEDIUM

- `deploy-pages` serves assets via a CDN. HTML is sent with a short/no-cache TTL, but
  hashed/static assets can be cached aggressively; the real risk is the browser holding a
  **stale `tte_wasm_bg.wasm`/`tte_wasm.js`** after a redeploy.
- The wasm-bindgen glue references `tte_wasm_bg.wasm` by a fixed name (no content hash), so
  a redeploy reuses the same URLs → stale-cache risk if a browser cached the old `.wasm`.
- Mitigations (pick one):
  1. **Cache-bust via query string** — easiest: in `index.html`, import `app.js` (and have
     app import pkg) with `?v=2.1.0`, bumped per release. Low effort; the wasm URL is built
     by the glue from `import.meta.url`, so to bust the wasm too you'd pass an explicit URL
     to `init(new URL('./pkg/tte_wasm_bg.wasm?v=2.1.0', ...))`.
  2. **Hashed filenames** — most robust (content hash in filename → new URL each change,
     immutable caching safe). Requires a post-`wasm-bindgen` rename + rewrite step; more
     build complexity than this small demo warrants.
- Pragmatic call for v2.1.0: a release-version query param (`?v=<crate version>`) on the
  module imports is sufficient; revisit hashing only if stale-wasm reports appear.
- Sources:
  - https://rustwasm.github.io/book/reference/deploying-to-production.html
  - https://github.com/actions/deploy-pages (CDN-served Pages deployment)

---

## RECOMMENDED DEPLOY WORKFLOW

### Repo-settings + .nojekyll + relative-path checklist
- [ ] Settings → Pages → Source = **GitHub Actions** (one-time, manual).
- [ ] Add empty file **`web/.nojekyll`** (committed). Critical.
- [ ] Confirm relative paths (already true): `index.html` → `./app.js`; `app.js` →
      `./pkg/tte_wasm.js`, `./renderer.js`. No leading `/`.
- [ ] Optional: cache-bust module imports with `?v=2.1.0` per release (Q7).

### `.github/workflows/deploy.yml`
```yaml
name: Deploy demo to Pages
on:
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

# Serialize deploys; don't cancel an in-flight publish.
concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@v2
        with:
          # Pin to the crate's wasm-bindgen; binaryen gives the wasm-opt -Oz size pass.
          tool: wasm-bindgen@0.2.123,wasm-opt
      - name: Build wasm + bindings (also enforces NFR-7 size budget)
        run: ./web/build.sh
      - name: Ensure Jekyll is disabled
        run: touch web/.nojekyll
      - uses: actions/configure-pages@v5
      - uses: actions/upload-pages-artifact@v5
        with:
          path: web

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

### CI smoke test (add to existing `ci.yml`, runs on PR + push)
```yaml
  demo-smoke:
    name: Demo smoke (Playwright)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@v2
        with:
          tool: wasm-bindgen@0.2.123,wasm-opt
      - run: ./web/build.sh
      - run: npx --yes playwright@latest install --with-deps chromium
      - name: Serve + smoke test
        run: |
          (cd web && python3 -m http.server 8000 &) && sleep 2
          node scripts/smoke.mjs   # goto localhost:8000, fail on console/pageerror,
                                    # wait for #status to match /\d+×\d+ · \d+ FPS/
```
`scripts/smoke.mjs`: launch chromium, capture `console`/`pageerror`, `page.goto`,
`page.waitForFunction(() => /\d+×\d+ · \d+ FPS/.test(document.querySelector('#status')?.textContent))`,
exit non-zero on any error. ~30 lines.

### PR previews verdict
Skip for v2.1.0. The official artifact flow can't multiplex previews; the only path is the
legacy `gh-pages`-branch model (`rossjrw/pr-preview-action`), which is a separate publish
mechanism and security posture. The CI Playwright smoke test covers per-PR confidence.

