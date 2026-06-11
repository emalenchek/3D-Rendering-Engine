# MVP Scoping: From-Scratch Software 3D Rasterizer (terminal ASCII → WASM)

Status: research in progress (incremental notes). Date: 2026-06-11.

## 1. tinyrenderer (ssloy) — canonical precedent
- Claim: a complete software renderer in **~500 lines of code** ("software rendering in 500 lines of code" is the project tagline). HIGH — https://github.com/ssloy/tinyrenderer
- Reported effort: README states students typically need **10–20 hours of programming** to produce such a renderer. HIGH — https://github.com/ssloy/tinyrenderer (README)
- Lesson sequence (= dependency-ordered feature list). HIGH — https://github.com/ssloy/tinyrenderer/wiki
  1. Bresenham line drawing
  2. Triangle rasterization + back-face culling
  3. Hidden-face removal (z-buffer)
  4. Perspective projection
  5. Moving the camera (lookat / view matrix)
  6. Programmable-style "shaders" refactor (vertex/fragment abstraction)
  7. (extensions) tangent-space normal mapping, shadow mapping, ambient occlusion
- Input format: triangulated mesh (Wavefront OBJ) + textures; output is an image file (TGA) — no windowing/GUI in scope. HIGH

## 2. Other curricula — convergent minimal stage list
- Gambetta, *Computer Graphics from Scratch* (No Starch / free online), Part II Rasterization chapter order: Lines → Filled Triangles → Shaded Triangles → Perspective Projection → Scene description/rendering → Clipping → Hidden Surface Removal → Shading → Textures → Extending the Rasterizer. HIGH — https://gabrielgambetta.com/computer-graphics-from-scratch/
  - Note Gambetta adds **clipping** as an explicit stage that tinyrenderer skips; language-agnostic pseudocode, no libraries.
- Convergence across tinyrenderer + Gambetta: the universal core is {line draw, filled triangle, z-buffer, perspective projection, camera transform, per-vertex/per-pixel shading}; textures/normal maps/shadows are uniformly "phase 2+".

## 3. Documented effort data points (precedents)
- tinyrenderer: ~500 LOC, **10–20 hours** for students following lessons. HIGH — https://github.com/ssloy/tinyrenderer
- Trenki's software renderer (C++, vertex+pixel "shaders" as C++ classes, OpenMP, fixed-point): documented as a multi-part dev blog (4 parts over ~1 month of posts, Jun–Jul 2017); features: vertex processor, viewport clipping, back-face culling, arbitrary varyings interpolation; fixed-point gave 10–20% gain on triangle fill. MEDIUM (blog 403'd; data via search snippets) — https://trenki2.github.io/blog/2017/06/06/developing-a-software-renderer-part1/ , https://github.com/trenki2/SoftwareRenderer
- krzosa/software_rasterizer: full-featured CPU rasterizer (AVX2+FMA, 8 px/iter, tile-based multithreading) renders **Sponza at 30 FPS on a Ryzen 5800U**. HIGH — https://github.com/krzosa/software_rasterizer

## 4. CPU rasterization performance reference points
- Intel Software Occlusion Culling sample (the codebase of Fabian Giesen's "Optimizing Software Occlusion Culling" series): SSE + multithreading gives **up to ~8x speedup** vs naive; Giesen's rasterizer-side changes alone added ~13% frame rate / ~27% cull-time improvement on the already-optimized sample. MEDIUM-HIGH — https://fgiesen.wordpress.com/2013/02/17/optimizing-sw-occlusion-culling-index/ , https://software.intel.com/content/www/us/en/develop/articles/software-occlusion-culling.html
- Practical implication: a scalar, unoptimized rasterizer is roughly an order of magnitude slower than a SIMD+MT one — but see terminal-resolution arithmetic below.
- **Resolution arithmetic (key MVP insight):** a 200x50 terminal frame = 10,000 cells; 1920x1080 = 2,073,600 px → terminal target is **~200x fewer pixels** than 1080p. Even a naive scalar rasterizer that manages only 10 Mpix/s of fill would fill a 10k-cell frame in ~1 ms (1000 fps of headroom). Rasterization speed is NOT the MVP risk; terminal escape-code emission and stdout flushing are. (verification pending)

