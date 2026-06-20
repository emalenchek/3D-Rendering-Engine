# Dependency License Compliance Audit (v2.0 tree)

Date: 2026-06. Tool: `cargo deny check licenses` + `cargo deny list` (cargo-deny 0.19.9).
Scope: the full transitive dependency graph across all crates/targets/features.

## Result: ✅ `licenses ok`

Every dependency — direct and transitive, across native and wasm targets — is under a
**permissive** license already on the project's allow-list (`deny.toml`). **No copyleft**
(no GPL/LGPL/AGPL/MPL/EPL) anywhere in the tree.

## Licenses actually present (what we redistribute)

| SPDX | Notes | Examples |
|---|---|---|
| `MIT` | most crates are dual MIT/Apache | crossterm, rayon, wasm-bindgen, syn, libc, … |
| `Apache-2.0` | most crates are dual MIT/Apache | rayon, num-traits, approx, wasm-bindgen, … |
| `Apache-2.0 WITH LLVM-exception` | (dual with MIT) | rustix, linux-raw-sys, wasi |
| `Unicode-3.0` | data-license clause; carried by `unicode-ident` (itself `(MIT OR Apache-2.0) AND Unicode-3.0`) | unicode-ident |

All four are OSI-approved / FSF-permissive and impose only **attribution** obligations
(reproduce copyright + license text when redistributing). None impose source-disclosure.

## Notes

- The `deny.toml` allow-list also lists `BSD-2-Clause`, `BSD-3-Clause`, `Zlib` which are
  **not currently used** (cargo-deny warns "license was not encountered"). Harmless; keep
  `Zlib` because the planned v2.1 SIMD dependency `wide` is `Zlib OR Apache-2.0 OR MIT`.
- Dev-dependencies (criterion, proptest, insta, assert_cmd, predicates, expectrl,
  wasm-bindgen-test) are likewise permissive and are **not** shipped in the binary/wasm, so
  they carry no distribution-time attribution obligation.

## Distribution-time obligation (binary + Pages-hosted wasm)

Because MIT/Apache/BSD/Unicode all require preserving each dependency's copyright + license
notice when **distributing** compiled artifacts, the released `tte` binary and the
Pages-hosted `tte_wasm_bg.wasm` should ship a generated **THIRD-PARTY-LICENSES** manifest.
Recommended tool: `cargo-about` (or `cargo-bundle-licenses`). (Confirm/finalize against
research report 13.)

## Re-run

```sh
cargo deny check licenses     # compliance gate (already in CI)
cargo deny list               # per-license inventory
```
