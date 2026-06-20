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
  `Copyright (c) 2026 Evan Malenchek` (or your preferred public handle). A handle
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
