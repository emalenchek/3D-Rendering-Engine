# MVP Scoping: From-Scratch Software 3D Rasterizer (terminal ASCII → WASM)

Date: 2026-06-11. Method: web research on documented precedents (tinyrenderer, Gambetta, Scratchapixel, weekend-rasterizer blogs, terminal-perf threads). Note: many primary blogs returned 403 to automated fetch; those claims are marked MEDIUM and sourced from search snippets/secondary mirrors.

## 0. Data table — concrete LOC / effort / perf data points

| Precedent | Scope | LOC | Effort | Perf | Source |
|---|---|---|---|---|---|
| ssloy/tinyrenderer | lines→triangles→z-buffer→perspective→camera→shaders (+textures, normal/shadow maps) | ~500 C++ | 10–20 h (students, stated in README) | n/a (renders to TGA, offline) | https://github.com/ssloy/tinyrenderer |
| Kayhan "Rasterization in One Weekend" | edge-fn triangle → 3D + depth buffer → perspective-correct attribs | n/s | ~1 weekend (basic); tile-based optimized version was a separate later project | n/s | https://tayfunkayhan.wordpress.com/2018/11/24/rasterization-in-one-weekend/ |
| lisyarus "tiny CPU rasterizer" | clear screen → full rasterizer → optimizations | n/s | 12 blog parts | n/s | https://lisyarus.github.io/blog/posts/implementing-a-tiny-cpu-rasterizer.html |
| Trenki software renderer | vertex/pixel shader classes, clipping, culling | n/s | 4-part blog over ~1 month | fixed-point: +10–20% fill rate | https://trenki2.github.io/blog/2017/06/06/developing-a-software-renderer-part1/ |
| krzosa/software_rasterizer | full scenes, AVX2+FMA, tiled MT | n/s | n/s | Sponza @ 30 FPS, Ryzen 5800U | https://github.com/krzosa/software_rasterizer |
| Intel/Giesen SW occlusion culling | SIMD+MT depth rasterizer | n/s | n/s | SSE+MT ≈ up to ~8x vs naive | https://fgiesen.wordpress.com/2013/02/17/optimizing-sw-occlusion-culling-index/ |
| Windows Terminal #10362 | terminal output cost | — | — | per-cell color SGR ~40x slower than plain text | https://github.com/microsoft/terminal/issues/10362 |
| Terminal emulator throughput | — | — | — | modern emulators ~30–100 MB/s; legacy ≥10x less | https://lobste.rs/s/k2mjsk/how_fast_should_unoptimized_terminal_run |
| Target frame budget | 200x50 truecolor frame | — | — | ≈200 KB/frame → ~12 MB/s @60fps (derived) | (arithmetic, this doc) |

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
- Scratchapixel, "Rasterization: a Practical Implementation" — stage list: projection stage → rasterization stage (edge function / coverage) → visibility problem via depth buffer + depth interpolation (interpolate 1/z, then invert) → perspective-correct vertex-attribute interpolation. HIGH — https://www.scratchapixel.com/lessons/3d-basic-rendering/rasterization-practical-implementation/rasterization-practical-implementation.html
- Convergence across tinyrenderer + Gambetta + Scratchapixel + Kayhan: the universal core is {filled triangle via edge functions or scanline, z-buffer (interpolating 1/z), perspective projection, camera/lookat transform, per-vertex or per-pixel diffuse shading, perspective-correct attribute interpolation}; line drawing is a Lesson-1 stepping stone (wireframe milestone); textures/normal maps/shadows/clipping are uniformly "phase 2+". Only Gambetta treats near-plane clipping as a core chapter; tinyrenderer and the weekend builds skip or fudge it (an MVP can clamp/cull near-plane-crossing triangles initially). HIGH (convergent)

## 3. Documented effort data points (precedents)
- tinyrenderer: ~500 LOC, **10–20 hours** for students following lessons. HIGH — https://github.com/ssloy/tinyrenderer
- Trenki's software renderer (C++, vertex+pixel "shaders" as C++ classes, OpenMP, fixed-point): documented as a multi-part dev blog (4 parts over ~1 month of posts, Jun–Jul 2017); features: vertex processor, viewport clipping, back-face culling, arbitrary varyings interpolation; fixed-point gave 10–20% gain on triangle fill. MEDIUM (blog 403'd; data via search snippets) — https://trenki2.github.io/blog/2017/06/06/developing-a-software-renderer-part1/ , https://github.com/trenki2/SoftwareRenderer
- krzosa/software_rasterizer: full-featured CPU rasterizer (AVX2+FMA, 8 px/iter, tile-based multithreading) renders **Sponza at 30 FPS on a Ryzen 5800U**. HIGH — https://github.com/krzosa/software_rasterizer

### 3b. More effort data points
- Tayfun Kayhan, "Rasterization in One Weekend" (2018): basic but modern rasterizer built in ~a weekend, in 3 parts — Part I "Hello, Triangle!" (edge-function rasterization to 2D screen), Part II "Go 3D!" (3D objects + depth buffer, initially without perspective divide), Part III (vertex attributes + perspective-correct interpolation). Performance issues then motivated a separate follow-up project (Tyler, tile-based rasterizer, 2019) — i.e., naive-first then optimize was a months-later separate effort, not part of the weekend. MEDIUM-HIGH (search snippets; blog blocks fetch) — https://tayfunkayhan.wordpress.com/2018/11/24/rasterization-in-one-weekend/ , https://tayfunkayhan.wordpress.com/2019/07/26/chasing-triangles-in-a-tile-based-rasterizer/
- lisyarus, "Implementing a tiny CPU rasterizer" (2024): 12-part C++ series from clearing the screen through full rasterization to optimizations — confirms that "tiny" + optimization is a ~12-installment effort even for an experienced graphics blogger. MEDIUM (could not fetch; series index + HN thread exist) — https://lisyarus.github.io/blog/posts/implementing-a-tiny-cpu-rasterizer.html , https://news.ycombinator.com/item?id=42017726

## 4. CPU rasterization performance reference points
- Intel Software Occlusion Culling sample (the codebase of Fabian Giesen's "Optimizing Software Occlusion Culling" series): SSE + multithreading gives **up to ~8x speedup** vs naive; Giesen's rasterizer-side changes alone added ~13% frame rate / ~27% cull-time improvement on the already-optimized sample. MEDIUM-HIGH — https://fgiesen.wordpress.com/2013/02/17/optimizing-sw-occlusion-culling-index/ , https://software.intel.com/content/www/us/en/develop/articles/software-occlusion-culling.html
- Practical implication: a scalar, unoptimized rasterizer is roughly an order of magnitude slower than a SIMD+MT one — but see terminal-resolution arithmetic below.
- **Resolution arithmetic (key MVP insight):** a 200x50 terminal frame = 10,000 cells; 1920x1080 = 2,073,600 px → terminal target is **~200x fewer pixels** than 1080p. Even a naive scalar rasterizer that manages only 10 Mpix/s of fill would fill a 10k-cell frame in ~1 ms (1000 fps of headroom). Rasterization speed is NOT the MVP risk; terminal escape-code emission and stdout flushing are.

### 4b. Terminal I/O as the real bottleneck — evidence
- Windows Terminal issue #10362: per-character color output (SGR escape per cell) measured **~40x slower** than single-color output — escape-sequence density, not pixel count, dominates. HIGH — https://github.com/microsoft/terminal/issues/10362
- Lobsters/"How fast should an unoptimized terminal run": modern terminal emulators sustain **~30–100 MB/s** input throughput; legacy emulators ≥10x less; the bottleneck is the **single-threaded ANSI escape-state-machine parsing**, not glyph rasterization. MEDIUM (search snippet; thread 403'd on fetch) — https://lobste.rs/s/k2mjsk/how_fast_should_unoptimized_terminal_run
- Counterpoint: "A completely unoptimized terminal renderer that runs at several thousand FPS" (HN, casey muratori refterm discussion) — terminal-side rendering CAN be fast; slowness is an emulator-implementation property the app can't control. MEDIUM — https://news.ycombinator.com/item?id=27728177
- Sizing: a 200x50 frame with truecolor SGR per cell ≈ 10k cells x ~20 bytes ≈ **200 KB/frame**; at 60 fps that's ~12 MB/s — within modern-emulator budgets but near legacy-emulator limits. Mitigations used by TUI libs: diff-based updates (only changed cells), run-length merging of identical SGR attributes, single buffered write + flush per frame, alternate screen + cursor-home instead of clear. MEDIUM (derived; consistent with sources above and with 02-ascii-terminal-rendering.md)

## 5. Scope creep / "write games, not engines"
- Josh Petrie, "Write Games, Not Engines" (2007, mirrored at geometrian): build a concrete game (here: a concrete demo — spinning shaded mesh in a terminal) with "well defined developmental scope"; extract the engine afterwards from working products, not before. "The notion that you must have an engine to build a non-trivial game is a fallacy." HIGH — https://geometrian.com/projects/blog/write_games_not_engines.html
- Implication for this MVP: define the deliverable as a demo ("load OBJ, orbit it, flat/Gouraud shaded, in a terminal"), not as "a rendering engine"; defer abstraction layers (material systems, scene graphs, plugin backends) until ≥2 concrete demos exist. — HIGH (direct application)

