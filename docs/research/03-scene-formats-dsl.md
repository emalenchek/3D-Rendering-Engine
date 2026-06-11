# Text-Based 3D Scene Description Formats — Prior Art for a Human-Writable Scene DSL

Research notes, 2026-06-11. Prior art survey for designing a human-writable scene DSL ("text in") for this engine. Claims carry source URLs and confidence (HIGH/MEDIUM/LOW).

## 1. Wavefront OBJ (+MTL)

- OBJ is a line-oriented keyword format: `v x y z`, `vn`, `vt`, `f v/vt/vn ...`, `o`/`g` for object/group names, `usemtl` + `mtllib` linking a companion .mtl file. No scene hierarchy, no transforms, no lights, no cameras, no animation, no units. It is a *geometry interchange* format, not a scene format. Confidence: HIGH (well-established; https://en.wikipedia.org/wiki/Wavefront_.obj_file)
- Why ubiquitous: trivially simple to emit and to partially parse (you can ignore lines you don't understand), plain text, stable since ~1990, supported by virtually every DCC tool. Its simplicity is exactly its limit: per-face material assignment and vertex positions only. Confidence: HIGH
- Parse complexity is low *conceptually* but performance-sensitive in practice: the cost is dominated by float parsing and line tokenization, which is why specialized parsers differ by ~100x (see §7). Confidence: HIGH (https://github.com/aras-p/obj_parse_tester)
- MTL is a separate, similarly flat keyword file (Ka/Kd/Ks/Ns/map_Kd...); it predates PBR, so modern PBR workflows bolt on unofficial extensions (Pr/Pm/Ke, etc.) — a lesson in what happens when a material model is frozen in the format. Confidence: HIGH

**Synthesis:** OBJ won on parse simplicity, text inspectability, and ignore-what-you-don't-know extensibility — but it deliberately describes *one mesh blob*, not a scene. A scene DSL should treat OBJ as an *asset import* target, never as a syntax model.

## 2. USD ASCII (.usda)

- .usda is the human-readable serialization of USD: nested `def Xform "name" { ... }` prims with typed attributes (`float3 xformOp:translate = (0,0,0)`), metadata in `(...)`, and relationships. Nesting expresses hierarchy directly. Confidence: HIGH (https://openusd.org/release/usdfaq.html, https://docs.nvidia.com/learn-openusd/latest/stage-setting/usd-file-formats.html)
- USD's power and weight come from **composition arcs** — six operators ("LIVRPS": subLayers, inherits, variantSets, references, payloads, specializes) that merge many layers with strength-ordered overrides. This enables non-destructive multi-artist pipelines but makes the *meaning* of a file non-local: you cannot know a prim's final value without composing the whole stage. Confidence: HIGH (https://github.com/ColinKennedy/USD-Cookbook/blob/master/concepts/asset_composition_arcs.md, https://www.sidefx.com/docs/houdini/solaris/glossary.html)
- NVIDIA's own docs position .usda as best for *small top-level files that reference external content*, and note it is bulky/slow for heavy data; binary .usdc is used for payloads. Human-writability is real for small scene-assembly files, poor for geometry. Confidence: HIGH (https://docs.nvidia.com/learn-openusd/latest/stage-setting/usd-file-formats.html)
- Pixar notes .usda is actually UTF-8, not pure ASCII. Confidence: MEDIUM (https://openusd.org/dev/api/_usd__page__u_t_f_8.html)

**Synthesis:** .usda demonstrates the right *shape* for a scene DSL — typed nested blocks, names, references — while also demonstrating the cost ceiling: composition semantics are where complexity explodes. Steal the nested-prim syntax; do not steal LIVRPS.

## 3. POV-Ray SDL

- SDL is a Turing-complete scene language: primitives, CSG (`union`/`intersection`/`difference`/`merge`), `#declare`, parameterized `#macro`s, `#while`/`#for` loops, conditionals, even file I/O (`#fopen`/`#write`). Confidence: HIGH (https://en.wikipedia.org/wiki/POV-Ray, https://wiki.povray.org/content/HowTo:Use_macros_and_loops)
- The cost of that power: parsing *is* program execution. Nested loops/macros "dramatically increase the parse time"; the community workaround is using `#write` to generate flat include files — which the POV wiki itself notes become hard to read and debug. Confidence: HIGH (https://wiki.povray.org/content/HowTo:Use_macros_and_loops, https://github.com/POV-Ray/povray/issues/313)
- Tooling consequence: POV-Ray's parser is a giant hand-written recursive-descent C++ unit with no stable external AST; third-party editors/engines support only subsets of SDL because full support means reimplementing an interpreter. Confidence: MEDIUM (https://github.com/POV-Ray/povray/blob/master/source/parser/parser.cpp, https://infinity3dengine.com/pov-ray-sdl-support-status/)
- Positive lesson: decades of hobbyists happily hand-wrote whole scenes in SDL — nested curly-brace blocks (`sphere { <0,1,2>, 0.5 texture {...} }`), good primitive defaults, and an include library of named textures/colors made it *pleasant*. Confidence: MEDIUM (community history; https://wiki.povray.org/)

**Synthesis:** SDL proves people enjoy hand-writing scenes when syntax is nested-block, defaults are good, and a standard library of named assets exists. It also proves Turing-completeness kills tooling and makes parse time unbounded. Declarative data + external codegen beats an embedded ad-hoc language.

## 4. OpenSCAD

- Loved: programmatic, fully parametric CSG; modules as reusable named parts; many users build parametric models faster than in GUI CAD. Confidence: HIGH (https://learn.cadhub.xyz/blog/openscad-review/)
- Hated: "variables" are compile-time constants — reassignment is silently accepted with counter-intuitive results; functional semantics hidden under procedural-looking syntax ("iterative programmers disease"); no structs/dictionaries; single-expression functions; janky scoping in loops. Confidence: HIGH (https://github.com/openscad/openscad/issues/4301, https://forum.openscad.org/What-s-your-opinion-on-these-criticisms-of-Openscad-td13866.html)
- Pressure to replace the language is constant (openscad issue #4763 "New Language?", microcad project) — evidence the pain is the syntax/semantics mismatch, not the CSG model. Confidence: MEDIUM (https://github.com/openscad/openscad/issues/4763, https://hackaday.com/2025/11/26/microcad-programs-cad/)

**Synthesis:** Same lesson as POV-Ray from the other side: a half-language (looks imperative, is functional) frustrates everyone. If users need programmability, give them a real host language (cf. CadQuery/build123d in Python) that *emits* the declarative format.

## 5. glTF (JSON)

- glTF 2.0 = JSON scene graph (nodes with TRS transforms, meshes, materials, cameras, animations, skins) + binary buffers; "JPEG of 3D" positioning: a *last-mile transmission* format optimized for runtime loading, explicitly not an authoring format. Confidence: HIGH (https://en.wikipedia.org/wiki/GlTF, https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)
- Key machine-efficiency trick: JSON header describes typed `accessors`/`bufferViews` over binary blobs, so geometry/animation arrays go from wire to GPU without parsing. Confidence: HIGH (https://github.khronos.org/glTF-Tutorials/gltfTutorial/gltfTutorial_002_BasicGltfStructure.html)
- Hand-writability tradeoffs: the JSON is readable/diffable (names, hierarchy visible in any editor), but authoring by hand is hostile — everything is index-based cross-references (`"mesh": 3`, `"material": 0`), data must be base64-encoded or in a sidecar .bin, JSON has no comments, and accessors require byte-offset bookkeeping. Minimal hand-written glTF samples (e.g. a single triangle) take ~50 lines plus base64. Confidence: HIGH (https://github.khronos.org/glTF-Tutorials/gltfTutorial/gltfTutorial_003_MinimalGltfFile.html, https://docs.fileformat.com/3d/gltf/)

**Synthesis:** glTF nails the *content model* a scene DSL needs (node hierarchy + TRS + cameras + lights ext + PBR materials) and proves JSON-with-indices is the wrong surface syntax for humans. Use glTF as compile target / import format; mirror its content model with friendlier syntax (names instead of indices, nesting instead of flat arrays).

## 6. Brief precedents

### VRML / X3D
- VRML97 was a full human-writable text scene graph (nested nodes, `DEF`/`USE` for named reuse, sensors/routes for behavior) — first web 3D standard, 1995. Confidence: HIGH (https://en.wikipedia.org/wiki/VRML)
- It failed for ecosystem reasons (dial-up bandwidth, plugin dependence, no native browser GPU access, authoring complexity), not primarily syntax; X3D (XML re-encoding, 2001) inherited the niche but never went mainstream either. WebGL+glTF won instead. Confidence: MEDIUM-HIGH (https://novedge.com/blogs/design-news/design-software-history-from-vrml-to-webgl-the-history-of-3d-on-the-web, https://www.web3d.org/x3d-vrml-most-widely-used-3d-formats)
- Durable ideas worth stealing: `DEF name` / `USE name` (define-once, instance-many), nested Transform nodes for hierarchy, sensible defaults on every field. Confidence: HIGH (VRML97 spec, well established)

### Mermaid / Graphviz DOT — successful human-writable graphics DSLs
- Mermaid succeeded by being markdown-adjacent, minimal-syntax, diffable, and rendered natively where developers already are (GitHub/GitLab/docs tools). Concise human authoring beat Graphviz's greater power/layout control for the documentation use case. Confidence: HIGH (https://www.unidiagram.com/blog/mermaid-vs-graphviz-comparison, https://medium.com/softaai-blogs/what-is-mermaid-a-complete-guide-to-the-text-based-diagramming-language-developers-love-3cc5dfff66b9)
- DOT lesson: a tiny declarative core (`a -> b [label="x"]`) with attribute lists has survived 30+ years and is emitted by countless programs — declarative data formats become *targets for codegen*, which Turing-complete ones cannot cleanly be. Confidence: MEDIUM (https://news.ycombinator.com/item?id=14636163, general knowledge)
- Both also turned out to be excellent LLM input/output formats precisely because they are small, regular, and textual — increasingly relevant for "text in" pipelines. Confidence: MEDIUM (https://tonywood.co/blog/mermaid-diagrams-as-shared-language-for-humans-and-agents)

### KDL / RON / TOML as syntax bases
- KDL: node-based (not key-value) document language — nodes have a name, positional args, key=value properties, and an optional `{ }` children block; comments; repeatable same-name siblings. This maps 1:1 onto a scene graph (`node "lamp" { light intensity=2.0; transform translate=(0,1,0) }`) without the map-key contortions JSON/TOML force. Confidence: HIGH (https://kdl.dev/, https://github.com/kdl-org/kdl)
- TOML handles deep nesting badly (multiple competing syntaxes, depth-dependent forms) — poor fit for hierarchies. JSON lacks comments and is strict/noisy. YAML's significant whitespace becomes unmanageable in large files. Confidence: HIGH (https://lobste.rs/s/zhzxfg/things_i_don_t_like_configuration, https://kdl.dev/)
- RON (Rusty Object Notation) adds named structs/enums/tuples to a JSON-like syntax; used by Bevy/Amethyst for scene serialization, but Bevy judged raw reflection-serialized RON scenes too verbose for hand-authoring and is building BSN ("Bevy Scene Notation"): a terse, nested, human-composable scene format with scene inheritance/patching, designed so "scenes must be human composable" (PRs #20158, #23413; discussion #14437). Confidence: HIGH (https://github.com/bevyengine/bevy/discussions/14437, https://github.com/bevyengine/bevy/pull/20158, https://github.com/ron-rs/ron)
- Godot's .tscn is a deliberately human-readable, VCS-diffable text scene format (INI-like sections describing a node tree + named sub-resources) — widely cited as a reason Godot projects merge well in git. Confidence: HIGH (https://docs.godotengine.org/en/4.4/contributing/development/file_formats/tscn.html)

## 7. Parse speed — is parsing the interactive bottleneck?

- Aras Pranckevičius's obj_parse_tester (2022, Ryzen 5950X / M1 Max) benchmarked 7 C++ OBJ parsers. On a 2.5GB Blender splash scene: rapidobj 1.25s (~2 GB/s, multithreaded), fast_obj ~5s single-threaded (~485 MB/s on M1), tinyobjloader ~128 MB/s, assimp ~22s, OSG ~26 MB/s. On 20MB Sponza: rapidobj 0.02s. Confidence: HIGH (https://github.com/aras-p/obj_parse_tester, https://aras-p.info/blog/2022/05/14/comparing-obj-parse-libraries/)
- tinyobjloader — the "default choice" — is among the slower options; its multithreaded variant trades memory (13.8GB peak on the 2.5GB file) for speed. Confidence: HIGH (same sources)
- simdjson parses JSON at multiple GB/s on commodity cores (project claims gigabytes/second, ~4x faster than RapidJSON), via SIMD + two-stage parsing. Confidence: HIGH (https://github.com/simdjson/simdjson, https://arxiv.org/abs/1902.08318)
- Counter-example worth remembering: POV-Ray scenes can take minutes to "parse" — but that is macro/loop *execution*, not tokenization. Parsing is only slow when the format embeds computation. Confidence: HIGH (https://wiki.povray.org/content/HowTo:Use_macros_and_loops)
- Implication: for *hand-written* scene files (KB–low MB), parse time is microseconds-to-milliseconds with any sane parser — never the interactive bottleneck. Parsing only matters for bulk geometry, which argues for keeping heavy mesh data *out* of the DSL (reference external OBJ/glTF/binary blobs instead). Confidence: HIGH (inference from above numbers)

**Synthesis:** Parsing a human-scale DSL file is free. Engineering effort should go into (a) a fast bulk-geometry import path (rapidobj-class, multithreaded, or binary cache after first import) and (b) good error messages, not parser micro-optimization.

## 8. Design lessons for pleasant hand-written scene DSLs

Distilled from the formats above (each lesson tagged with its evidence source):

- **Defaults everywhere.** VRML gave every field a default; POV-Ray primitives render with one line. A sphere should be `sphere {}` — unit radius at origin, default material. Verbosity is the #1 hand-authoring killer (glTF's index/offset bookkeeping is the anti-pattern). Confidence: HIGH (VRML97 spec; glTF minimal-file tutorial)
- **Nesting = hierarchy.** Every pleasant format (usda, VRML, POV SDL, KDL, BSN, tscn-ish) expresses parent/child by literal block nesting, so the file's indentation *is* the scene tree. Flat node arrays with index references (glTF) are machine-friendly, human-hostile. Confidence: HIGH
- **Names, not indices; define-once / use-many.** VRML `DEF`/`USE`, POV `#declare`d textures, USD references, Godot named sub-resources, Bevy BSN inheritance: named reusable assets (materials, meshes, prototypes) at file top, instanced by name in the tree. Confidence: HIGH
- **Comments are non-negotiable** (the most-cited JSON-for-config complaint). Confidence: HIGH (https://kdl.dev/, lobste.rs config-language threads)
- **Declarative data, not embedded programming.** POV-Ray and OpenSCAD show ad-hoc Turing-completeness ruins tooling, parse-time bounds, and learnability; users who need loops should generate the format from Python/Rust (the Graphviz DOT model: simple declarative target, infinite generators). Confidence: HIGH (§3, §4, §6)
- **One transform convention, stated in the file format docs and never varied.** TRS per node (glTF model: translate, then rotate, then scale, applied child-of-parent), one handedness, one up-axis, degrees for human input. USD's flexible xformOp stacks and OBJ's absent units both create friction. Confidence: MEDIUM (glTF spec; USD xformOp complexity)
- **Diff/VCS-friendliness drives adoption** (Mermaid, Godot .tscn): stable ordering, line-oriented values, no gratuitous re-serialization churn. Confidence: HIGH
- **Small regular grammars double as LLM-friendly formats** — Mermaid's surge in agent workflows suggests "text in" scene DSLs will be machine-*written* as often as hand-written. Confidence: MEDIUM
- **Separate the interchange path from the authoring path.** Every successful stack splits them: usda interface layers vs usdc payloads; glTF JSON vs .bin; Godot .tscn vs imported assets. Heavy meshes enter via `mesh "file.obj"` references, never inline. Confidence: HIGH

## Implications for our DSL — recommendation

1. **Build a small custom nested-block DSL, with KDL as the syntax skeleton.** Concretely: adopt KDL's node grammar (node name, positional args, `key=value` props, optional `{children}`, `//` comments) either by literally using a KDL parser library (kdl-rs etc. exist for most languages — free parsing, editor highlighting, spec'd escaping) or by hand-rolling an equivalent ~300-line recursive-descent parser if we want scene-specific sugar (e.g. bare `(x,y,z)` vectors, degree literals). KDL semantics (repeatable named nodes, child blocks) map exactly onto a scene graph in a way TOML/JSON/YAML do not. RON is a reasonable fallback only if the engine is Rust and we want serde round-tripping; it is more verbose for trees.
2. **Content model: a simplified glTF.** Node tree with TRS transforms; primitives (sphere/box/plane/mesh-ref); PBR-ish material with few required fields; camera; lights. Named top-level `material`/`mesh`/`prototype` definitions referenced by name (VRML DEF/USE spirit). Defaults for every field.
3. **Strictly declarative.** No loops/macros/expressions in v1. Programmatic scenes are produced by a tiny Python/Rust builder API that emits the DSL (DOT/Graphviz model). If repetition pressure appears later, add a constrained `instance ... count=` array node, not a language.
4. **OBJ (and glTF) import is a separate asset path, not part of the DSL.** The DSL references external geometry (`mesh "models/teapot.obj"`); the importer uses a fast parser (rapidobj/fast_obj class) and caches to a binary representation. Never inline mesh data in scene text.
5. **Spend the saved effort on error messages and hot-reload.** Parse speed is a non-issue at hand-written scale (§7); friendly diagnostics with line/column and "did you mean", plus file-watch reload, are what make a text-in workflow feel good (the Mermaid lesson: tight write-render loop beats expressive power).

Sketch of target feel:

```kdl
scene "demo" up="y" {
    material "red" base-color=(0.8 0.1 0.1) roughness=0.4
    camera position=(0 2 8) look-at=(0 1 0) fov=60
    light "sun" type="directional" direction=(-1 -2 -1) intensity=3

    node "table" translate=(0 0 0) {
        box "top" size=(2 0.1 1) translate=(0 1 0) material="red"
        node "legs" { /* ... */ }
    }
    mesh "teapot" src="assets/teapot.obj" translate=(0 1.05 0) scale=0.5
}
```
