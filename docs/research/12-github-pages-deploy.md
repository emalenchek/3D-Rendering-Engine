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
