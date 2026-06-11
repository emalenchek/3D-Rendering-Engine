# Research: 3D Engine Architectures & API Design

Status: COMPLETE
Date: 2026-06-11
Scope: inform the design of a minimal software-rendering 3D engine (ASCII terminal output + browser via WASM).
Method note: direct page fetches were blocked (HTTP 403) for most domains in this environment; findings come from WebSearch result summaries cross-checked against prior knowledge. Claims marked with confidence HIGH/MEDIUM/LOW accordingly.

## Q1: Three.js object model

- Every three.js app is built from three scaffolding objects: **Scene, Camera, Renderer**; you call `renderer.render(scene, camera)` each frame. (HIGH)
  - Sources: https://discoverthreejs.com/book/first-steps/first-scene/ , https://threejs.org/manual/
- The Scene is a container ("tiny universe") holding everything visible; visible objects are **Mesh = Geometry + Material**. Geometry defines shape; Material defines surface appearance ("color of the pixels"). (HIGH)
  - Sources: https://discoverthreejs.com/book/first-steps/first-scene/ , https://medium.com/@gianluca.lomarco/three-js-basic-scene-b749c5516fc6
- `BufferGeometry` stores vertex data in typed arrays that map directly to GPU buffers; it replaced the older object-per-vertex `Geometry` for performance. Lesson: store vertex data as flat typed arrays, not object graphs. (HIGH)
  - Sources: https://threejs.org/docs/pages/BufferGeometry.html , https://57blocks.com/blog/improve-performance-on-threejs-scenes-using-custom-meshes-and-buffergeometry?tab=engineering
- `MeshBasicMaterial` is the simplest material and is unlit (needs no lights) — i.e., a usable engine can ship a no-lighting material first and add lit materials later. (HIGH)
  - Source: https://discoverthreejs.com/book/first-steps/first-scene/
- The camera is *not* part of the visible scene; it defines the viewpoint passed to render(). (HIGH)
  - Source: https://medium.com/@gianluca.lomarco/three-js-basic-scene-b749c5516fc6
- Everything placeable derives from `Object3D`, which carries position/quaternion/scale, `visible`, a `children` array, and a lazily-composed local + world matrix; Scene, Mesh, Camera, Light, Group are all Object3D subclasses, giving one uniform transform-hierarchy mechanism. Scene itself is the Object3D root. (HIGH — confirmed via search summaries)
  - Sources: https://threejs.org/docs/ (Object3D page), https://github.com/mrdoob/three.js/issues/8711
- The Object3D conflation (transform + hierarchy + render-related flags in one base class) is convenient but has known ambiguity costs (long-standing issue discussing Object3D vs Group vs Mesh overlap). (MEDIUM)
  - Source: https://github.com/mrdoob/three.js/issues/8711

**Q1 synthesis:** The proven minimal retained-mode set: Renderer, Scene (root node), Node (transform + children), Mesh(Geometry, Material), Camera, Light. Geometry = flat typed arrays of attributes + optional index. Single entry point `render(scene, camera)`; the frame loop is user-owned.

## Q2: Lower-level engines

### Filament (Google)
- Setup objects: **Engine** (owns all resources), **Renderer**, **SwapChain** (output target). Per-frame you need a **View = Scene (what) + Camera (from where) + viewport/quality settings (how)**. (HIGH)
  - Sources: https://google.github.io/filament/dup/intro.html , https://github.com/google/filament , https://medium.com/@philiprideout/getting-started-with-filament-on-android-d10b16f0ec67
- **No classic scene graph node type.** Filament uses an entity-component model: an Entity is just an ID; a TransformManager component gives it a transform (transforms *can* be composed into a tree via the transform component, but the Scene itself is a flat set of entities, not a hierarchy). A RenderableManager component makes an entity drawable. Lights are likewise components. (HIGH)
  - Sources: https://google.github.io/filament/dup/intro.html ("Filament does not provide a 'node' type like a classic scene graph; instead it provides transformable components that can be composed into a tree"), https://deepwiki.com/google/filament
- glTF loader pattern confirms this: each glTF node → entity + transformable component; node-with-mesh → also gets a renderable component. (HIGH)
  - Source: https://google.github.io/filament/ (gltfio docs via search)
- Instructive split: **Scene = membership container** (flat), **transform hierarchy = separate optional facility**, **View = scene+camera+settings binding**. Decoupling "what's in the world" from "how it's parented" keeps the renderer's hot loop a flat iteration. (HIGH)

### bgfx
- "Bring Your Own Engine" library: graphics-API-agnostic draw-submission layer over ~11 backends; deliberately NOT an engine — no scene graph, no materials, no loaders. (HIGH)
  - Source: https://github.com/bkaradzic/bgfx (README)
- Declarative **View** concept: user declares numbered views (render target + clear params + view/proj transforms) up front, then submits draw calls in any order; bgfx sorts internally. Each draw is encoded into a **64-bit sort key** (grouped by view, then program, then depth) to minimize state changes — draw-call sorting instead of a scene traversal order. (HIGH)
  - Sources: https://bkaradzic.github.io/bgfx/internals.html , https://bkaradzic.github.io/bgfx/bgfx.html
- All resources are opaque **handles** (vertex/index buffer, texture, program), not pointers/objects. Draw state is transient: cleared after every `bgfx::submit` — each draw fully re-specifies its state. (HIGH)
  - Sources: https://bkaradzic.github.io/bgfx/bgfx.html , https://github.com/bkaradzic/bgfx/blob/master/include/bgfx/bgfx.h
- **Encoder** model: one encoder per thread for parallel draw submission; internally double-buffered Frame objects (submit buffer / render buffer) let the API thread and render thread run in parallel. (HIGH)
  - Sources: https://bkaradzic.github.io/bgfx/internals.html , https://github.com/bkaradzic/bgfx/issues/1282
- Lesson for a small engine: a render core that consumes a flat, sortable list of stateless draw commands is simple, testable, and independent of any scene structure. Threading/encoders are deferrable complexity. (HIGH — design inference)

### raylib
- KISS-principle C library "to enjoy videogames programming"; inspired by Borland BGI and XNA; no external docs needed — API is a one-page cheatsheet of self-explanatory functions. (HIGH)
  - Sources: https://github.com/raysan5/raylib , https://www.raylib.com/cheatsheet/cheatsheet.html
- **Immediate-mode drawing**: frame loop is `BeginDrawing() … EndDrawing()`; 3D goes inside `BeginMode3D(camera) … EndMode3D()` with calls like `DrawCube`, `DrawGrid`, `DrawModel`. No scene graph at all — the user's own code *is* the scene. Camera is a plain struct passed to BeginMode3D. (HIGH)
  - Sources: https://learnxinyminutes.com/raylib/ , https://www.adacore.com/blog/ada-gamedev-part-3-enjoy-video-games-programming-with-raylib-2 , https://www.raylib.com/cheatsheet/cheatsheet.html
- Lesson: an immediate-style `Draw*` layer is the fastest path to "something on screen" and trivially testable; retained scene structure can be layered on top later. Also: cheatsheet-sized API surface is itself a design goal. (HIGH — design inference)

### wgpu (prior knowledge, MEDIUM — well-established public API, not re-verified by fetch)
- WebGPU/wgpu's model: `Instance → Adapter → Device + Queue`; resources (Buffer, Texture, Sampler, ShaderModule) created from Device; immutable precompiled **RenderPipeline** objects capture all draw state; **BindGroups** bundle resource bindings; per-frame a **CommandEncoder** records RenderPasses which issue `set_pipeline/set_bind_group/set_vertex_buffer/draw`, then the command buffer is submitted to the Queue.
  - Source: https://www.w3.org/TR/webgpu/ , https://docs.rs/wgpu
- Lesson: modern APIs moved state into immutable pipeline objects + explicit per-frame command recording; even a software renderer benefits from "validate/derive everything once at creation, keep the per-draw path dumb."

### OGRE (classic scene graph)
- OGRE separates renderable objects from placement: you create an **Entity** (mesh instance) via the **SceneManager** and attach it to a **SceneNode**; the scene is a hierarchy of SceneNodes giving relative transforms; SceneManager owns/creates everything (nodes, entities, lights, cameras). (HIGH)
  - Sources: https://ogrecave.github.io/ogre/api/1.10/tut__first_scene.html , https://ogrecave.github.io/ogre/api/1.10/class_ogre_1_1_scene_manager.html
- The Node/attached-MovableObject split (transform carrier vs renderable content) is the classic alternative to three.js's "Mesh is-a Object3D" inheritance: composition over inheritance, at the cost of more API ceremony for simple scenes. (MEDIUM — design inference from the above)

## Q3: Scene graph vs ECS vs flat lists

- **Tom Forsyth, "Scene Graphs — Just Say No" (2006)**: homogeneous classic scene graphs are "of little value in modern games" — the renderer has no reason to be driven by the transform hierarchy; a flat list will probably run faster. Hugely influential in moving engines away from monolithic graphs. (HIGH)
  - Sources: https://tomforsyth1000.github.io/blog.wiki.html#%5B%5BScene%20Graphs%20-%20just%20say%20no%5D%5D , https://gamedev.net/forums/topic/464464-anti-scenegraphism-a-tale-of-tom-forsyths-scene-graphs-just-say-no/ , https://forums.ogre3d.org/viewtopic.php?t=35895
- The mature consensus (mindcontrol.org "The Scene Graph Argument", gamedev.net threads): split concerns into separate structures — a **SpatialGraph** (culling/visibility), a **SceneTree** (parent-child transforms, only where actually needed e.g. skeletal/attached objects), and a flat **RenderQueue** the renderer consumes. Most objects in a game don't need a deep hierarchy at all. (HIGH)
  - Sources: http://www.mindcontrol.org/~hplus/graphics/scene-graph.html , https://www.gamedev.net/forums/topic/636166-alternatives-to-a-scene-graph/ , http://diaryofagraphicsprogrammer.blogspot.com/2009/01/handling-scene-geometry.html
- Filament embodies this in production: flat Scene of entities; transform hierarchy is an optional component facility, not the render-driving structure. (HIGH — see Q2/Filament sources)
- Bevy (prior knowledge, MEDIUM): pure ECS engine; hierarchy expressed as Parent/Children components with a transform-propagation system computing `GlobalTransform` from `Transform` — i.e., hierarchy is data + one system pass, the renderer iterates flat queries. Source: https://bevy.org/learn/
- For a *small* engine the cost calculus differs: a shallow scene graph (three.js style) is the simplest mental model for users and fine at small object counts; the harm Forsyth describes (cache misses, forced traversal order, kitchen-sink nodes) bites at scale. (MEDIUM — synthesis)

**Q3 synthesis:** Keep the user-facing model a simple tree (familiar, ergonomic), but internally flatten: each frame, walk the tree once to produce world matrices + a flat draw list; the rasterizer consumes only the flat list. Avoid making the graph the renderer's data structure.

## Q4: Software rasterizer pipeline (tinyrenderer + Scratchapixel)

- **tinyrenderer** (ssloy) is the canonical "write OpenGL from scratch" course: a complete software rasterizer in ~500 lines / 10–20 hours, structured as: model loading → line drawing → triangle rasterization via barycentric coordinates over a bounding box → z-buffer → perspective projection → ModelView/Projection/Viewport matrices → programmable vertex+fragment "shader" structs. (HIGH)
  - Sources: https://github.com/ssloy/tinyrenderer , https://github.com/ssloy/tinyrenderer/wiki/Lesson-6:-Shaders-for-the-software-renderer , https://haqr.eu/tinyrenderer/rasterization/
- tinyrenderer's final architecture is instructive: a fixed `triangle()` rasterizer that takes an `IShader` with `vertex(face, vert) -> clip coords` and `fragment(barycentric) -> color/discard`. The pipeline core never changes; effects live in shader objects. (HIGH)
  - Source: https://github.com/ssloy/tinyrenderer/wiki/Lesson-6:-Shaders-for-the-software-renderer
- Rasterization inner loop (both sources agree): per triangle, compute screen-space bounding box (clamped to framebuffer), iterate pixels, compute barycentric coords / edge functions; any negative component ⇒ outside, skip. (HIGH)
  - Sources: https://github.com/ssloy/tinyrenderer , https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/rasterization-stage.html
- **Edge function method** (Scratchapixel, after Pineda): point-in-triangle = sign of cross-product edge function for all 3 edges; the same edge function values, normalized, ARE the barycentric coordinates — one computation serves coverage + interpolation. Top-left rule resolves pixels exactly on shared edges. (HIGH)
  - Source: https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/rasterization-stage.html
- **Perspective-correct interpolation** (Scratchapixel): screen-space barycentrics interpolate attributes incorrectly under perspective; correct method ("rational-linear / hyperbolic interpolation"): pre-divide each vertex attribute by vertex z (or multiply by 1/w), interpolate attribute/z and 1/z linearly in screen space with barycentrics, then divide: `attr = (Σ λi·attri/zi) / (Σ λi/zi)`. Depth itself: interpolate 1/z linearly, then invert. (HIGH)
  - Sources: https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/rasterization-practical-implementation.html , https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/5222204
- **Z-buffer**: per-pixel depth array, init to far/∞; write fragment only if its interpolated depth is nearer; resolves visibility with no sorting. (HIGH)
  - Sources: https://github.com/ssloy/tinyrenderer , https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/rasterization-stage.html
- **Near-plane clipping must happen in homogeneous clip space, BEFORE the perspective divide.** Vertices at/behind the camera (w≤0 or z near 0) blow up to infinity / flip sign after division, producing "external triangles" and giant coordinates. Clip each triangle against the near plane (Sutherland–Hodgman against w=ε or z=-near), yielding 0, 1, or 2 triangles, then divide. tinyrenderer largely sidesteps this (models framed safely in front of camera) — a real engine cannot. (HIGH)
  - Sources: https://www.gamedeveloper.com/business/in-depth-software-rasterizer-and-triangle-clipping , https://gamedev.net/forums/topic/434666-nearfar-clipping-in-a-software-rasterizer/ , http://simonstechblog.blogspot.com/2012/04/software-rasterizer-part-1.html , https://handmade.network/forums/t/7743-3d_software_rasterizer._triangle_clipping_problem
- X/Y clipping, by contrast, can be replaced by bounding-box clamping ("scissoring") to the framebuffer — only the near plane strictly requires geometric clipping. (MEDIUM — widely stated in the gamedev threads above and standard practice)
- Optimization note for later (Scratchapixel): edge functions are incremental (constant delta per pixel step) and amenable to 8x8 block testing — relevant if terminal-resolution performance ever becomes an issue (unlikely: ~80×24 to ~300×100 cells). (MEDIUM)
  - Source: https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/rasterization-stage.html

**Q4 synthesis — canonical stage list for our rasterizer:**
1. Model matrix: object→world (from scene tree flatten)
2. View matrix: world→camera (inverse of camera node's world transform)
3. Projection matrix: camera→clip space (4D homogeneous)
4. Back-face cull (optional, signed area test — free byproduct of edge function)
5. Near-plane clip in homogeneous coords (Sutherland–Hodgman, 0–2 output triangles)
6. Perspective divide by w → NDC
7. Viewport transform → screen/raster space (mind non-square terminal cells: fold cell aspect ratio in here)
8. Per-triangle: bbox clamp → edge-function/barycentric loop → 1/z depth test vs z-buffer → perspective-correct attribute interpolation → fragment shading → write char/color
The vertex-shader/fragment-shader split (tinyrenderer Lesson 6) is the right internal seam even if we never expose user shaders.

## Q5: MVP API surface vs deferrable

- three.js demonstrates the floor for a *usable* retained API: Scene, PerspectiveCamera, Mesh(BufferGeometry, Material), Renderer.render(scene, camera) — its own "hello cube" uses exactly these plus nothing else, with an unlit material and no lights. (HIGH)
  - Source: https://discoverthreejs.com/book/first-steps/first-scene/
- raylib demonstrates the floor for an *immediate* API: camera struct + Begin/End + Draw* primitives; entire library fits a cheatsheet. (HIGH)
  - Sources: https://www.raylib.com/cheatsheet/cheatsheet.html , https://github.com/raysan5/raylib
- Filament/bgfx demonstrate that the render core should not know about scene structure: it should consume flat draw lists / renderable sets. (HIGH — Q2 sources)
- tinyrenderer demonstrates the complete feature ladder and its order: flat-shaded triangles → z-buffer → perspective → camera → Gouraud → textures → normal maps → shadows. Everything after "Gouraud + z-buffer" is optional polish. (HIGH)
  - Source: https://github.com/ssloy/tinyrenderer

**MVP needs:** mesh from vertex/index arrays; transform (position/rotation/scale) per object; perspective camera; directional + ambient light; flat or Gouraud shading (Lambert N·L); z-buffer; near clipping; render-to-buffer (the ASCII/colour mapping is a presentation layer on top of a float/intensity framebuffer).
**Defer:** textures/UVs (high effort, low payoff at terminal resolution), material system beyond "color + shading mode", point/spot lights, skeletal/any animation (user mutates transforms per frame instead), shadows, frustum culling beyond near-clip + bbox clamp, multithreading/encoders, spatial acceleration structures.

## Implications for our MVP

1. **Two-layer architecture, stolen from the Forsyth consensus + Filament:** (a) user-facing retained scene: tiny three.js-style tree — `Node {transform, children}`, `Mesh : Node {geometry, material}`, `Camera : Node`, `Light : Node`, `Scene = root`; (b) internal flat pipeline: per frame, one tree walk emits world matrices + a flat array of draw commands `{world_matrix, geometry_ref, material}`; the rasterizer consumes only this array and never sees the tree. This gives three.js ergonomics without scene-graph-as-renderer pathology.
2. **One entry point**: `renderer.render(scene, camera) -> framebuffer`. The frame loop belongs to the caller (terminal loop natively; requestAnimationFrame in WASM). Renderer owns framebuffer + z-buffer; output is an intensity/color grid, with ASCII glyph mapping as a separate presentation stage — this same seam serves both terminal and browser/WASM targets.
3. **Geometry = flat typed arrays** (positions, normals, optional colors + index buffer), BufferGeometry-style — trivially serializable, WASM-friendly, no per-vertex objects.
4. **Pipeline stages fixed, shading pluggable**: implement the 8-stage list from Q4 synthesis as a fixed pipeline with an internal vertex-transform/fragment-shade seam (tinyrenderer Lesson 6 pattern). Do NOT skip homogeneous near-plane clipping — it is the one stage tutorials omit that breaks real interactive cameras.
5. **Keep the API cheatsheet-sized** (raylib lesson): if the public surface doesn't fit on one page, it's too big for this project. Optionally add 2–3 raylib-style convenience constructors (cube, sphere, plane) so "hello cube" is <10 lines.
6. **Defer aggressively**: no textures, no animation system, no ECS, no threading. Each has a clear later seam (material struct, per-frame transform mutation, flat draw list already ECS-compatible, bgfx-style encoder split) so deferring now costs nothing architecturally.

## Sources consulted (primary)

- three.js docs/manual: https://threejs.org/docs/ , https://discoverthreejs.com/book/first-steps/first-scene/
- bgfx: https://github.com/bkaradzic/bgfx , https://bkaradzic.github.io/bgfx/internals.html
- Filament: https://google.github.io/filament/dup/intro.html , https://github.com/google/filament
- raylib: https://github.com/raysan5/raylib , https://www.raylib.com/cheatsheet/cheatsheet.html
- OGRE: https://ogrecave.github.io/ogre/api/1.10/tut__first_scene.html
- Scene-graph debate: Tom Forsyth "Scene Graphs — Just Say No" (https://tomforsyth1000.github.io/blog.wiki.html), http://www.mindcontrol.org/~hplus/graphics/scene-graph.html
- Rasterizer: https://github.com/ssloy/tinyrenderer (+ wiki Lessons 0–6), https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/rasterization-stage.html , https://www.gamedeveloper.com/business/in-depth-software-rasterizer-and-triangle-clipping
