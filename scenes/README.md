# scenes/

Generated and hand-authored `tte` scenes live here; OBJ assets in `scenes/assets/`.

- Author/edit with the **`tte-scene`** skill (`.claude/skills/tte-scene/`), which knows
  the DSL/OBJ grammar and validates output.
- Format reference: `.claude/skills/tte-scene/reference.md`.
- Preview any file:
  ```sh
  cargo run -p tte-cli -- view scenes/your.scene            # interactive
  .claude/skills/tte-scene/validate.sh scenes/your.scene    # headless ASCII frame
  ```
