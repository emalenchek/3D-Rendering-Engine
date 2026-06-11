# Research: 3D Engine Architectures & API Design

Status: IN PROGRESS (incremental notes; will be polished at end)
Date: 2026-06-11
Method note: WebFetch was blocked (HTTP 403) for all domains in this environment; findings come from WebSearch result summaries plus prior-knowledge claims marked accordingly. Confidence ratings reflect this.

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

## Q3: Scene graph vs ECS vs flat lists

(to be filled)

## Q4: Software rasterizer pipeline

(to be filled)

## Q5: MVP API surface

(to be filled)
