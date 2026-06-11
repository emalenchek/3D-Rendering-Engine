# Text-Based 3D Scene Description Formats — Prior Art for a Human-Writable Scene DSL

Research notes (in progress). Date: 2026-06-11.

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

## 7. Parse speed — is parsing the interactive bottleneck?

- Aras Pranckevičius's obj_parse_tester (2022, Ryzen 5950X / M1 Max) benchmarked 7 C++ OBJ parsers. On a 2.5GB Blender splash scene: rapidobj 1.25s (~2 GB/s, multithreaded), fast_obj ~5s single-threaded (~485 MB/s on M1), tinyobjloader ~128 MB/s, assimp ~22s, OSG ~26 MB/s. On 20MB Sponza: rapidobj 0.02s. Confidence: HIGH (https://github.com/aras-p/obj_parse_tester, https://aras-p.info/blog/2022/05/14/comparing-obj-parse-libraries/)
- tinyobjloader — the "default choice" — is among the slower options; its multithreaded variant trades memory (13.8GB peak on the 2.5GB file) for speed. Confidence: HIGH (same sources)
- simdjson parses JSON at multiple GB/s on commodity cores (claimed >3 GB/s, ~4x faster than RapidJSON). Confidence: HIGH (https://github.com/simdjson/simdjson — to verify)
- Implication: for *hand-written* scene files (KB–low MB), parse time is microseconds-to-milliseconds with any sane parser — never the interactive bottleneck. Parsing only matters for bulk geometry, which argues for keeping heavy mesh data *out* of the DSL (reference external OBJ/glTF/binary blobs instead). Confidence: HIGH (inference from above numbers)

<!-- TODO: POV-Ray, OpenSCAD, glTF, VRML/X3D, Mermaid/Graphviz, KDL/RON/TOML, design lessons, implications -->
