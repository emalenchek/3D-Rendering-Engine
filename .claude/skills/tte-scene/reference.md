# tte scene & model reference

Authoritative, code-derived spec for what the engine accepts. Sources:
`crates/tte-core/src/dsl.rs` (scene DSL), `obj.rs` (OBJ), `primitives.rs`,
`scene.rs` (defaults). When in doubt, the parser is the source of truth — validate.

## Conventions

- **Right-handed**, **+Y up**; the camera looks down **−Z** in view space.
- **Colors are 0–1 floats** per channel (clamped); e.g. `(0.85 0.15 0.15)`.
- **Rotations are in degrees**, per axis: `rotate=(x y z)`.
- **Transform order is T·R·S**; child nodes inherit their parent's transform.
- Numbers are `f32`. Node separators are a newline or `;`.

## Scene DSL (`.scene`)

A strict, declarative, KDL-style grammar. **No loops, variables, or expressions** —
emit explicit nodes. An optional `scene { … }` wrapper is allowed; bare top-level
nodes also parse.

### Directives (scene-level)

| Directive | Syntax | Default if omitted |
|---|---|---|
| Background | `background color=(r g b)` | black |
| Material | `material "name" base-color=(r g b)` | (no materials); a node with no material is light-gray `(0.82 0.82 0.82)` |
| Camera | `camera position=(x y z) look-at=(x y z) fov=<deg>` | position `(3 2.2 4.5)`, look-at `(0 0 0)`, fov `50` |
| Light | `light direction=(x y z) intensity=<n> ambient=<n>` | direction `(-0.5 -1 -0.8)`, intensity `1.0`, ambient `0.15` |

Notes:
- There is exactly **one directional light**. `direction` is the direction the light
  **travels**, so a surface whose normal faces **−direction** is the most lit. `ambient`
  is a floor in `0..1` so back faces aren't pure black.
- A scene's own `camera` is used for headless/scene rendering; the interactive viewer
  orbits instead.

### Geometry nodes

Each may start with an optional `"name"`, then properties, then an optional `{ children }`.

| Node | Syntax | Geometry |
|---|---|---|
| Cube | `cube` (alias `box`) | unit cube, edges **±0.5** (size 1), origin-centered |
| Sphere | `sphere [rings=<n>] [segments=<n>]` | **unit radius** (Ø 2!), default `rings=12 segments=24` |
| Plane | `plane` | unit square in the **XZ** plane (±0.5), normal +Y — a ground |
| Mesh | `mesh src="file.obj"` | external OBJ; **`src` is required** |
| Group | `node` | no geometry; a transform container for `{ children }` |

⚠️ A `sphere` is **radius 1** while a `cube` is **edge 1** — a default sphere is ~2× a
default cube. Use `scale=0.5` on spheres to match cube-sized objects.

### Per-node properties (all optional)

| Prop | Syntax | Meaning |
|---|---|---|
| name | leading `"name"` | label (first positional string) |
| translate | `translate=(x y z)` | position offset |
| rotate | `rotate=(x y z)` | rotation in **degrees** per axis |
| scale | `scale=<n>` or `scale=(x y z)` | uniform or per-axis |
| material | `material="name"` | reference a `material` defined above |
| children | `{ … }` | nested nodes (inherit this node's transform) |

### Comments

`// line comment` and `/* block comment */`.

### Gotchas (all are line-numbered errors)

- Unknown element name (e.g. `teapot`) → `unknown scene element`.
- `mesh` without `src=` → error. Geometry node given a child block must use `{ }`.
- Browser/WASM demo has **no filesystem** → `mesh src=` renders **nothing** there.
  For browser-targeted scenes, use only `cube`/`sphere`/`plane`.
- `material` must be **defined before** it is referenced for the color to resolve.

### Minimal example

```kdl
scene {
    background color=(0.02 0.02 0.05)
    material "red" base-color=(0.85 0.15 0.15)
    camera position=(4 3 6) look-at=(0 0.5 0) fov=50
    light direction=(-1 -2 -1) intensity=1.2 ambient=0.18

    plane scale=(8 1 8)                              // ground (8×8)
    sphere "ball" translate=(-1.5 0.6 0) scale=0.6 material="red"
    node "tower" translate=(0 0.5 -1.5) {
        cube
        cube translate=(0 1.1 0) scale=(0.7 0.7 0.7) material="red"
    }
}
```

## Wavefront OBJ (`.obj`) — supported subset

Geometry only; **color comes from the scene `material`**, not the OBJ.

| Record | Supported |
|---|---|
| `v x y z` | yes (positions) |
| `vn x y z` | yes (normals; **optional** — derived from faces if absent) |
| `vt …` | parsed but ignored |
| `f …` | yes — corners `v`, `v/vt`, `v//vn`, `v/vt/vn`; 1-based **and** negative indices; **n-gons fan-triangulated** (must be convex) |
| `#` | comment |
| `o g s usemtl mtllib l p` | ignored |

Author meshes **centered near the origin at ~unit scale** (the scene `mesh` node's
transform positions/sizes them). Reference from a scene with:
`mesh src="assets/thing.obj" translate=(…) scale=… material="…"` (path is resolved
next to the `.scene` file).

### Minimal OBJ (a square pyramid)

```obj
# square pyramid, base on the XZ plane, apex at +Y
v -0.5 0 -0.5
v  0.5 0 -0.5
v  0.5 0  0.5
v -0.5 0  0.5
v  0   1  0
f 1 2 3 4      # base (quad → 2 tris)
f 1 2 5
f 2 3 5
f 3 4 5
f 4 1 5
```

## Validate & preview

Deterministic headless render to ASCII (no TTY needed), errors carry line numbers:

```sh
cargo run -q -p tte-cli -- view --headless --frames 1 --size 80x40 \
  --render solid --shading gouraud --mode ascii path.scene
```

Or the bundled wrapper: `./.claude/skills/tte-scene/validate.sh path.scene [WxH]`.
Modes: `--mode ascii|truecolor|halfblock`, `--render solid|wireframe`,
`--shading flat|gouraud`. Interactive look: `cargo run -p tte-cli -- view path.scene`.
