---
name: tte-scene
description: Author and validate scenes (.scene DSL) and models (.obj) for the tte 3D rendering engine in this repo. Use when asked to create, generate, design, build, or edit a tte scene or model — a .scene file, a .obj mesh, or geometry/lighting/camera for the engine.
---

# Authoring tte scenes & models

This engine renders two text formats: the **scene DSL** (`.scene`) and **Wavefront
OBJ** (`.obj`, geometry only). `reference.md` (next to this file) is the authoritative,
code-derived spec — **read it before generating**; it lists every supported element,
the conventions (colors 0–1, angles in degrees, +Y up, camera looks −Z), and the
gotchas. Do not invent elements or properties — the parser rejects unknowns with a
line number.

## Workflow

1. **Clarify** the request just enough: what objects, rough layout, mood/lighting,
   camera framing, and whether it must run in the **browser demo** (then primitives
   only — `mesh src=` can't load there). Pick sensible defaults rather than
   over-asking.
2. **Generate** the `.scene` (and any small `.obj`) using `reference.md`. Prefer
   built-in primitives (`cube`/`sphere`/`plane`) and `node` groups; use OBJ only for
   custom shapes. Remember a `sphere` is radius 1 vs a `cube` edge 1 — scale to match.
3. **Validate + preview** (do this every time, before claiming it works):
   - build once: `cargo build -q -p tte-cli`
   - run: `./.claude/skills/tte-scene/validate.sh <file> [WxH]`
   - it parse-checks and prints an ASCII frame. Fix any line-numbered error and
     iterate until it renders the intended composition. **Paste the ASCII preview**
     into your reply so the user sees the result.
4. **Save** generated scenes under `scenes/` and OBJ assets under `scenes/assets/`
   (reference them with `mesh src="assets/thing.obj"`).
5. Offer the live look: `cargo run -p tte-cli -- view <file>` (orbit with arrows/hjkl,
   `q` to quit), or load it into the browser demo for primitive-only scenes.

## Rules

- Only the elements/properties in `reference.md`. Colors are 0–1; rotations in degrees.
- Define a `material` before referencing it.
- `light direction` is the direction light travels — surfaces facing the opposite way
  are the most lit; keep some `ambient` so shadowed faces aren't pure black.
- For browser-targeted scenes, avoid `mesh src=` (no filesystem in the demo).
- Always validate; never hand back a scene you haven't rendered.

## Examples

`examples/` has known-good starting points — copy and adapt:
- `ground-and-primitives.scene` — plane + spheres + a box, lit.
- `grouped-tower.scene` — nested `node` groups and transforms.
- `custom-obj.scene` + `assets/pyramid.obj` — a `mesh src=` reference.
