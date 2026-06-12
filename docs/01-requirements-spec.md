# Requirements Specification & Test Plan

Status: **Active** — covers environment staging (Phase 0) and Phase 1 (terminal wireframe).
Later phases are outlined and will be specified in detail when their phase begins.
Derived from [docs/00-project-brief.md](00-project-brief.md) §4–5; verification methods follow
[docs/research/06-rust-testing-best-practices.md](research/06-rust-testing-best-practices.md).

## 1. Conventions

- **Requirement IDs** are stable: `FR-<phase>.<n>` (functional), `NFR-<n>` (non-functional).
- **Traceability**: every automated test embeds the lowercased requirement ID in its
  function name (`fn fr1_4_...`). `cargo test fr1_4` runs exactly that requirement's tests;
  grepping for `fr1_4` finds its coverage.
- **Verification methods**:
  - *Unit* — `#[cfg(test)]` tests, may touch private internals.
  - *Property* — `proptest` invariants over generated inputs.
  - *Golden frame* — render a fixed scene/camera/size, snapshot the character frame with `insta`.
  - *E2E* — run the real `tte` binary via `assert_cmd`; golden frames flow through the
    `--headless` frame-dump mode (no PTY needed).
  - *PTY* — interactive-path tests via a pseudo-terminal (`expectrl`); used sparingly, Phase 3+.
- A requirement is **Done** only when its mapped tests exist and pass in CI.

## 2. Phase 0 — Environment staging (implemented)

| ID | Requirement |
|---|---|
| FR-0.1 | `tte --version` prints `tte <semver>` and exits 0 |
| FR-0.2 | `tte --help` (and bare `tte`) prints usage and exits 0 |
| FR-0.3 | Unknown arguments exit non-zero with an error naming the argument and pointing to `--help` |

## 3. Phase 1 — Terminal wireframe renderer

**Deliverable:** `tte view model.obj` shows the model spinning as a wireframe in the
terminal; `tte view --headless --size WxH --frames N model.obj` dumps frames as plain text.

| ID | Requirement |
|---|---|
| FR-1.1 | Math types `Vec3`/`Vec4`/`Mat4` with multiply, `look_at`, and `perspective`; correct by the invariants in the test plan |
| FR-1.2 | OBJ loader for the minimal subset: `v`/`vt`/`vn`/`f` records; 1-based and negative indices; fan triangulation of >3-gon faces; unknown line types ignored; missing normals derived |
| FR-1.3 | Projection pipeline: model→view→projection transform of mesh edges, near-plane culling of edges crossing the near plane, perspective divide, viewport transform folding in the terminal cell aspect ratio |
| FR-1.4 | Bresenham line rasterization of projected edges into a cell buffer of configurable size |
| FR-1.5 | Cell buffer is a pure value with a deterministic plain-text rendering (one line per row), independent of any terminal |
| FR-1.6 | Terminal presenter: alternate screen on entry/exit, cursor hidden, cursor-home (not clear) between frames, one buffered write + flush per frame |
| FR-1.7 | Animation loop: model rotates at a fixed angular velocity on a fixed timestep; `q`/Ctrl-C exits cleanly restoring the terminal |
| FR-1.8 | Headless mode: `--headless --size WxH --frames N` renders N frames to stdout as plain text with no ANSI escapes, deterministically |

### Non-functional requirements

| ID | Requirement | Verification |
|---|---|---|
| NFR-1 | Determinism: identical inputs (model, camera, size, frame index) produce byte-identical frames | Golden frame tests are themselves the check; double-render equality test |
| NFR-2 | Portability: workspace builds and tests pass on Linux, macOS, Windows | CI matrix |
| NFR-3 | Performance: headless render of a ≤1k-triangle model at 200×50 in ≤5 ms/frame on CI hardware (generous bound; terminal I/O excluded) | criterion bench, trend-tracked (not a hard CI gate) |
| NFR-4 | Code health: rustfmt-clean, clippy-clean (`-D warnings` in CI), no `unsafe` without justification | CI lint gate + workspace lint config |
| NFR-5 | Dependency hygiene: no known advisories; licenses within the allow-list | `cargo deny` CI gate |

## 4. Test plan (requirement → tests)

Status values: ✅ passing · 🚧 planned (test to be written with the feature).

| Req | Method(s) | Test name(s) / location | Status |
|---|---|---|---|
| FR-0.1 | E2E | `fr0_1_version_reports_crate_version` — `tte-cli/tests/e2e_cli.rs` | ✅ |
| FR-0.2 | E2E + golden | `fr0_2_help_output_matches_golden` — `tte-cli/tests/e2e_cli.rs` | ✅ |
| FR-0.3 | E2E | `fr0_3_unknown_argument_fails_with_message` — `tte-cli/tests/e2e_cli.rs` | ✅ |
| FR-1.1 | Unit + property | `fr1_1_*` in `tte-core/src/math.rs` (unit: look_at/perspective/cross sanity); proptest invariants: normalize length, cross orthogonality, mul associativity over vectors, rotation∘inverse ≈ I, rotation preserves length — `tte-core/tests/math_props.rs` | ✅ |
| FR-1.2 | Unit | `fr1_2_*` in `tte-core/src/obj.rs`: all four face index forms, negative indices, fan triangulation, junk-line tolerance, derived normals, line-numbered errors | ✅ |
| FR-1.3 | Unit + golden frame | `fr1_3_*`: center projection, behind-camera cull, near-plane-crossing cull (`tte-core/src/render.rs`); golden frames + camera-inside-cube — `tte-core/tests/render_wireframe.rs` | ✅ |
| FR-1.4 | Unit + golden frame | `fr1_4_*`: Bresenham cases (horizontal/vertical/diagonal/steep/clipped/point/reversed) — `tte-core/src/raster.rs`; rotated-cube golden — `tests/render_wireframe.rs` | ✅ |
| FR-1.5 | Unit | `fr1_5_*` in `tte-core/src/cell.rs`: Display shape (height lines × width chars), put/get, out-of-bounds safety | ✅ |
| FR-1.6 | Unit (byte-level) | `fr1_6_*` in `tte-cli/src/present.rs`: injected `Write` sink; asserts alt-screen enter/leave, cursor hide/show, cursor-home-not-clear, per-row addressing — no PTY | ✅ |
| FR-1.7 | Unit + PTY (smoke) | `fr1_7_*`: rotation-step determinism + quit-key mapping (unit); `fr1_7_interactive_quits_on_q` (`expectrl`, `#[ignore]`, unix) — `tte-cli/tests/e2e_render.rs` | ✅ |
| FR-1.8 | E2E + golden frame | `fr1_8_*` — `tte-cli/tests/e2e_render.rs`: headless golden frame, no-ANSI + frame-count check, missing-file & bad-size error paths; option parsing units in `tte-cli/src/lib.rs` | ✅ |
| NFR-1 | Integration + E2E | `nfr1_*`: double-render equality (lib) + byte-identical repeated CLI runs (e2e) | ✅ |
| NFR-2 | CI | test job matrix: ubuntu/macos/windows | ✅ |
| NFR-3 | Bench | `benches/raster.rs` (criterion): cube@80×24 ≈ 2.1 µs, 1k-tri sphere@200×50 ≈ 143 µs — 35× inside the ≤5 ms bound (2026-06, CI-class hardware) | ✅ |
| NFR-4 | CI | lint job (`fmt --check`, `clippy -D warnings`) | ✅ |
| NFR-5 | CI | deny job (`cargo deny check`) | ✅ |

### The functional e2e shape (template for all phases)

Every phase's headline requirement gets at least one test of this exact shape — the
full real path with deterministic inputs and a reviewable golden frame:

```text
fixture scene (tests/data/) ──> real `tte` binary, --headless, fixed size/camera/frame
                                          │
                                          ▼
                          plain-text frame on stdout (no ANSI)
                                          │
                                          ▼
                     insta::assert_snapshot! ──> committed .snap golden file
```

## 5. Later phases (outline — to be specified at phase start)

- **Phase 2 (shaded):** FR-2.x — edge-function fill, z-buffer, flat+Gouraud, ASCII ramp +
  half-block truecolor presenters. E2E: golden frames per shading mode and per presenter.
- **Phase 3 (interactive orbit):** FR-3.x — raw mode, orbit camera, resize. PTY tests enter scope.
- **Phase 4 (scene DSL):** FR-4.x — KDL-grammar parser, named materials, mesh refs,
  diagnostics, hot reload. Property round-trips + `cargo-fuzz` on the parser; golden frames per scene fixture.
