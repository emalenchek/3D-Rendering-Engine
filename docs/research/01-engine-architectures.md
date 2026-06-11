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
- (Prior knowledge, MEDIUM pending confirmation) Everything placeable derives from `Object3D`, which carries position/quaternion/scale, a `children` array, and a lazily-composed local + world matrix; Scene, Mesh, Camera, Light, Group are all Object3D subclasses, giving one uniform transform-hierarchy mechanism.
  - Source: https://threejs.org/docs/ (Object3D page)

## Q2: Lower-level engines

(to be filled)

## Q3: Scene graph vs ECS vs flat lists

(to be filled)

## Q4: Software rasterizer pipeline

(to be filled)

## Q5: MVP API surface

(to be filled)
