# 13 — Open-Source Licensing for the `tte` Workspace

Research report. **Not legal advice** — this gathers authoritative, well-sourced
guidance. Anywhere money, patents, or contributor agreements are at stake, a
real lawyer should confirm. Confidence tags: HIGH / MEDIUM / LOW.

Project facts (from repo, verified):
- Rust workspace, 3 crates: `tte-core` (lib), `tte-cli` (binary), `tte-wasm`
  (wasm-bindgen cdylib). Going public on GitHub for the first time (for Pages).
- `Cargo.toml` already declares `license = "MIT OR Apache-2.0"` (workspace.package).
- **No `LICENSE*` / `NOTICE` files exist yet** — the declaration is unbacked.
- `deny.toml` license allow-list: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception,
  BSD-2-Clause, BSD-3-Clause, Unicode-3.0, Zlib. Permissive-only, no copyleft.
- `web/` = hand-written JS frontend (`app.js`, `renderer.js`, `index.html`) + a
  `pkg/` wasm output dir. Not publishing to crates.io yet; keeping the option open.

---

## Q1 — MIT vs Apache-2.0 vs dual "MIT OR Apache-2.0"

- **Apache-2.0 has an explicit patent grant (§3); MIT does not.** Each
  contributor grants users a perpetual, royalty-free patent license to their
  contributions, and that grant **terminates** for anyone who initiates patent
  litigation over the software (patent-retaliation clause). MIT is silent on
  patents — protection only via an implied license, which is weaker/uncertain.
  Sources: HIGH.
  - https://www.apache.org/licenses/LICENSE-2.0 (§3 Grant of Patent License)
  - https://snyk.io/articles/apache-license/
  - https://fossa.com/blog/open-source-licenses-101-apache-license-2-0/
- **Apache-2.0 NOTICE requirement (§4):** if the work ships a `NOTICE` file,
  redistributors must reproduce its attribution content. NOTICE is *optional to
  create* but *mandatory to propagate if present*. MIT has no NOTICE concept —
  only "keep the copyright + permission notice." Source: HIGH.
- **Why the Rust ecosystem dual-licenses "MIT OR Apache-2.0":** this is the
  documented Rust-project default. Apache-2.0 supplies the patent grant + clear
  terms for derivative works; MIT is kept alongside because Apache-2.0's patent
  terms are **incompatible with GPLv2**, so offering MIT as an alternative keeps
  the crate usable by GPLv2 projects. "OR" = the downstream user picks whichever
  they want. Verified against 2 sources: HIGH.
  - Rust API Guidelines (necessities): recommends `MIT OR Apache-2.0`, the
    `LICENSE-APACHE`+`LICENSE-MIT` layout, and the README stanza below.
    https://rust-lang.github.io/api-guidelines/necessities.html
  - Rust internals "Rationale of Apache dual licensing":
    https://internals.rust-lang.org/t/rationale-of-apache-dual-licensing/8952
- **When plain MIT is preferable:** maximal simplicity, one short file, no NOTICE
  machinery; fine when patent exposure is negligible and ecosystem-convention
  matching is not a goal. MEDIUM.
- **Does the patent grant matter for a software rasterizer?** Rendering /
  rasterization has a history of patented techniques (historically S3TC texture
  compression, some shadow/AA methods). A *from-scratch CPU rasterizer* using
  textbook math is low-risk, but the Apache grant is cheap insurance and is the
  ecosystem norm — no reason to forgo it. MEDIUM (risk is judgment, not fact).

> The dual license is already declared in Cargo.toml; backing it (not downgrading
> to MIT-only) is the path of least surprise for a Rust project. See Recommendation.

---

## Q2 — What "MIT OR Apache-2.0" means + files to ship

- **SPDX semantics:** `MIT OR Apache-2.0` is a *disjunctive* expression. The `OR`
  operator means the downstream recipient **chooses either** license and complies
  with only that one. (Contrast `AND` = must satisfy both.) HIGH.
  - https://spdx.github.io/spdx-spec/v3.0.1/annexes/spdx-license-expressions/
  - https://fossa.com/blog/understanding-using-spdx-license-identifiers-license-expressions/
- **Files to back the declaration (Rust convention):** ship **both**
  `LICENSE-APACHE` and `LICENSE-MIT` at the repo root. This is exactly what the
  Rust API Guidelines prescribe; a single `LICENSE` cannot represent a choice of
  two. HIGH. (https://rust-lang.github.io/api-guidelines/necessities.html)
- **README license stanza** (verbatim from the API guidelines):
  ```
  ## License
  Licensed under either of
   * Apache License, Version 2.0
     ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
   * MIT license
     ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
  at your option.

  ## Contribution
  Unless you explicitly state otherwise, any contribution intentionally submitted
  for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
  dual licensed as above, without any additional terms or conditions.
  ```
  HIGH.
- **Does Apache-2.0 need a `NOTICE` file?** No — it is **optional**. You only need
  one if *you* want to assert attribution lines that downstream must carry. For a
  greenfield solo project with no upstream NOTICE obligations, you can skip it.
  If present, §4(d) forces redistributors to reproduce it. MEDIUM-HIGH.
  - https://www.apache.org/licenses/LICENSE-2.0 (§4)
  - https://infra.apache.org/licensing-howto.html (NOTICE is for required attributions only)

---

## Q3 — Copyright line / year / holder / SPDX headers

- **MIT `[year] [fullname]` placeholder:** put a real copyright line, e.g.
  `Copyright (c) 2026 Ethan Malenchek` (or your preferred public handle). A handle
  is legally acceptable; a real name is stronger for provenance. For a project
  expecting outside PRs, `Copyright (c) 2026 The tte contributors` is a common,
  low-maintenance choice (avoids enumerating contributors). MEDIUM.
- **Year:** the year of first publication; bump or use a range only if you care to.
  A single current year (2026) is fine and conventional. MEDIUM.
- **Apache `LICENSE-APACHE`:** ship the license text **unmodified**. The
  `Copyright [yyyy] [name]` appendix boilerplate is optional and usually omitted;
  the canonical Rust `LICENSE-APACHE` is the raw license body. MEDIUM-HIGH.
- **`SPDX-License-Identifier` headers in source files:** nice-to-have, not
  required. A one-line `// SPDX-License-Identifier: MIT OR Apache-2.0` per file
  makes per-file licensing machine-readable. Full **REUSE.software** compliance
  (a header in *every* file + `LICENSES/` dir) is **overkill** for a small solo
  project — the root LICENSE files + Cargo.toml field already satisfy tooling and
  GitHub detection. LOW-MEDIUM (judgment).
  - https://reuse.software/ (the strict standard; optional here)

---

## Q4 — Attribution obligations when redistributing permissive deps

The `tte-cli` binary and the `tte-wasm` artifact statically link permissive deps
(MIT / Apache / BSD / Zlib / Unicode-3.0). Distributing those artifacts =
"distributing in binary form," which triggers notice obligations:

- **MIT:** the copyright notice **and** the permission text must travel with all
  copies / substantial portions, including binaries. You don't have to advertise
  usage, but the notice must be retained "somewhere with the distribution." HIGH.
  - https://klarasystems.com/community/licensing/
- **BSD-2/3-Clause:** §"binary form" clause **explicitly** requires reproducing
  the copyright notice + condition list + disclaimer in the documentation/
  materials accompanying a binary distribution. Stricter/more explicit than MIT.
  HIGH. (https://opensource.org/license/bsd-3-clause, https://www.mend.io/blog/top-8-bsd-licenses-questions-answered/)
- **Apache-2.0 deps:** §4 — keep the license, state significant changes, and
  propagate any upstream `NOTICE` content. HIGH.
- **Unicode-3.0** (pulled by e.g. `unicode-*` data crates): permissive, GPL-
  compatible, but the copyright+permission notice must appear with copies **or**
  in associated documentation. So it must be carried in the third-party notices.
  MEDIUM-HIGH. (https://opensource.org/license/unicode-license-v3, https://spdx.org/licenses/Unicode-3.0.html)
- **Zlib:** unusually lenient — it does **not** require notice reproduction in
  binary distributions (only that you (1) don't misrepresent origin and (2) mark
  altered source versions; the notice clause applies to *source* distribution).
  Including it in your third-party file anyway is harmless and simplest. MEDIUM.
  (https://en.wikipedia.org/wiki/Zlib ; https://www.gnu.org/licenses/license-list.en.html)
- **Best practice — generate a bundled third-party-licenses file.** For a shipped
  binary *and* the Pages-hosted wasm, the accepted Rust practice is to generate an
  aggregated notices file and ship it alongside the artifact. HIGH that this is the
  norm; MEDIUM on exact tool choice.
  - `cargo-about` — template-driven, human-readable HTML/MD, integrates with a
    deny-style allow-list. Recommended primary. https://crates.io/crates/cargo-about
  - `cargo-bundle-licenses` — single bundled file, supports manual fill-in for
    licenses it can't auto-find. https://crates.io/crates/cargo-bundle-licenses
  - `cargo-3pl` / `cargo-attribution` — lighter alternatives.
  - Caveat: these tools are not lawyers; spot-check the output. MEDIUM.

---

## Q5 — Multi-crate workspace + JS/web frontend + GitHub

- **One license for the whole repo is fine.** All three crates already inherit
  `license = "MIT OR Apache-2.0"` from `[workspace.package]`. The root
  `LICENSE-MIT` + `LICENSE-APACHE` cover every crate and the `web/` JS. HIGH.
- **Hand-written `web/` JS (`app.js`, `renderer.js`, `index.html`):** it's your
  own code, so the repo-level dual license already governs it — no separate
  license needed. (Optional: a top-of-file SPDX comment for clarity.) MEDIUM.
- **The wasm artifact served on Pages** is a distributed binary → same Q4
  attribution duty as the native binary. Practically: publish the generated
  third-party-licenses file next to the demo (e.g. link it from the page or place
  it in the Pages output) so the wasm's bundled deps' notices travel with it.
  MEDIUM-HIGH.
- **GitHub specifics:** GitHub auto-detects a license and shows it in the repo
  "About" box, but its detector (Licensee) keys off a single `LICENSE`/`COPYING`
  file and may show "Other"/blank for the two-file `LICENSE-MIT`+`LICENSE-APACHE`
  layout. That's cosmetic — the Cargo metadata + the README stanza are the source
  of truth. You can optionally set the license field manually. MEDIUM.
  - https://docs.github.com/articles/licensing-a-repository

---

## Q6 — Copyleft sanity check (verified against this repo)

- **Verified the actual dependency tree** from `Cargo.lock` (182 packages). All
  are mainstream permissive crates: `rayon`, `wide`(not present — uses `wasm`
  SIMD via core), `wasm-bindgen`/`js-sys`/`web-sys`, `crossterm`, `approx`/
  `float-cmp`, `criterion`, `clap`, `serde`, `proptest`, `insta`, `tempfile`,
  `windows-*`, etc. **No GPL/LGPL/AGPL/MPL crate name appears.** HIGH.
- The `deny.toml` `[licenses].allow` list is permissive-only (MIT, Apache-2.0,
  Apache-2.0 WITH LLVM-exception, BSD-2/3-Clause, Unicode-3.0, Zlib) and
  cargo-deny **denies anything not on the list across the whole graph**, including
  transitive deps. This is the right guard. HIGH.
- **Why it matters:** a single GPL/AGPL transitive dep statically linked into the
  binary would create strong-copyleft obligations (you'd likely have to license
  the whole binary under the GPL/AGPL and provide corresponding source). The
  allow-list makes that a CI failure rather than a silent legal problem. HIGH.
  - https://embarkstudios.github.io/cargo-deny/checks/licenses/cfg.html
  - https://sts10.github.io/2023/04/18/cargo-deny-licenses.html

---

## Recommendation

**Keep the already-declared dual license `MIT OR Apache-2.0` — do not downgrade
to MIT-only.** Reasons:
1. It's already in `Cargo.toml`; backing it just means adding two files.
2. It's the Rust-ecosystem default (Rust API Guidelines + the Rust project),
   maximizing downstream compatibility and meeting contributor expectations.
3. Apache-2.0 adds an explicit **patent grant + retaliation** clause (cheap
   insurance for a rasterizer); MIT alongside preserves **GPLv2 compatibility**.
4. Zero downside for a not-yet-on-crates.io project; keeps the crates.io door open.

MIT-only is a *reasonable but weaker* alternative chosen purely for minimalism —
it would also mean editing `Cargo.toml` and `deny.toml` for consistency. Default
to the dual license.

> Non-lawyer framing: these are conventions and license-text readings, not legal
> advice. If patents or commercial redistribution become real concerns, consult
> counsel.

### Files to add (checklist)

- [ ] `LICENSE-MIT` at repo root — standard MIT text, first line:
      `Copyright (c) 2026 Ethan Malenchek` (or `The tte contributors`).
- [ ] `LICENSE-APACHE` at repo root — verbatim Apache-2.0 text
      (https://www.apache.org/licenses/LICENSE-2.0.txt), unmodified, no NOTICE needed.
- [ ] README "## License" + "## Contribution" stanza (verbatim text in Q2).
- [ ] (Optional) `// SPDX-License-Identifier: MIT OR Apache-2.0` header in source
      files — nice-to-have, skip full REUSE compliance.
- [ ] No `NOTICE` file required (no upstream NOTICE obligations of your own).

### Third-party attribution

- [ ] Generate a bundled third-party-licenses file for **both** the `tte-cli`
      binary and the `tte-wasm` Pages demo. Recommended: **`cargo-about`**
      (`cargo install cargo-about` → `cargo about generate about.hbs > THIRD-PARTY-LICENSES.html`).
      `cargo-bundle-licenses` is a fine alternative.
- [ ] Ship that file with the binary release **and** within/next to the Pages
      output so the wasm's bundled MIT/BSD/Apache/Unicode notices travel with it.
- [ ] Zlib needs no binary-form notice, but including it is harmless.
- [ ] Spot-check generated output (tools aren't lawyers).

### Sources (primary)
- Rust API Guidelines — https://rust-lang.github.io/api-guidelines/necessities.html
- Rust internals, dual-license rationale — https://internals.rust-lang.org/t/rationale-of-apache-dual-licensing/8952
- Apache-2.0 text — https://www.apache.org/licenses/LICENSE-2.0 ; howto https://infra.apache.org/licensing-howto.html
- SPDX expressions — https://spdx.github.io/spdx-spec/v3.0.1/annexes/spdx-license-expressions/
- OSI BSD-3 / Unicode-3.0 — https://opensource.org/license/bsd-3-clause , https://opensource.org/license/unicode-license-v3
- cargo-deny licenses — https://embarkstudios.github.io/cargo-deny/checks/licenses/cfg.html
- cargo-about / cargo-bundle-licenses — https://crates.io/crates/cargo-about , https://crates.io/crates/cargo-bundle-licenses
- GitHub licensing a repo — https://docs.github.com/articles/licensing-a-repository
