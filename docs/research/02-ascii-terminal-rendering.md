# Research: ASCII/Text 3D Rendering in Terminals — Prior Art & Performance Limits

Status: IN PROGRESS (incremental notes; will be polished at end)
Date: 2026-06-11

## Q1. ASCILINE (direct inspiration)

- **IMPORTANT CORRECTION: ASCILINE is NOT a 3D renderer.** It is "a high-performance, real-time ASCII *video* rendering engine" — it converts 2D video frames to text, it does not rasterize 3D geometry. Source: https://github.com/YusufB5/ASCILINE — confidence HIGH (fetched repo page directly).
- Languages: Python (63.9%), JavaScript (22.4%), CSS/HTML, Batchfile. Backend `stream_server.py` (FastAPI + uvicorn), frontend `app.js`/`index.html` render to HTML5 Canvas; also a standalone terminal player `ascii_video_player2.py`. HIGH.
- Pipeline: OpenCV decodes video → NumPy maps pixel data to ASCII character/color mappings → binary-encoded frames streamed over WebSocket → Canvas frontend. HIGH.
- Features: render modes B&W / 512 / 32K / 262K / 16M colors; "pixel mode" using colored blocks for "near-HD visual quality"; audio-synced master clock; 24–30 FPS target; playlists; FFmpeg server-side volume. HIGH.
- Run: `python stream_server.py video.mp4 --cols 240` (≈240 columns wide). HIGH.
- Takeaway for us: ASCILINE's relevant ideas are (a) NumPy-vectorized pixel→character mapping, (b) tiered color modes, (c) colored-block "pixel mode" for higher fidelity, (d) a web canvas frontend sidestepping terminal throughput limits. The 3D rasterization part is entirely ours to design.

## Q2. ASCII 3D renderers (donut.c etc.)

- donut.c (a1k0n, 2011 "Donut math", https://www.a1k0n.net/2011/07/20/donut-math.html): canonical technique. Core = framebuffer + per-cell z-buffer; torus is a circle swept/rotated around an axis, surface sampled at fixed angle increments densely enough to look solid (point-cloud splatting, NOT triangle rasterization); per point: perspective-project to a character cell, z-buffer test (stores 1/z), luminance = surface normal · light direction, mapped into 12-char ramp `.,-~:;=!*#$@` (dimmest→brightest). Confirmed via search results citing the post. Confidence HIGH. Sources: https://www.a1k0n.net/2011/07/20/donut-math.html, https://news.ycombinator.com/item?id=7108044
- TermGL (https://github.com/wojciech-graj/TermGL): C99 zero-dependency 2D/3D terminal graphics engine; full pipeline with custom vertex & pixel shaders, z-buffering, backface culling, affine texture mapping; 24-bit RGB and indexed (16 fg/16 bg + bold/underline) color modes; double-width character rendering option; non-blocking keyboard + mouse input; Windows & Unix. Demos include Utah teapot. No published perf numbers. HIGH (fetched repo).
- Rust crates: `rendersloth` (https://lib.rs/crates/rendersloth) — "one-of-a-kind Rust 3D renderer for the CLI", rasterizes triangles into "charxels" (character + color); `ascii_renderer` (https://lib.rs/crates/ascii_renderer) — wireframe-only renderer into a mutable CharBuffer (lines, fills, 3D wireframe). MEDIUM (search-level summaries, repos not fetched).
- Pattern across prior art: two families — (a) analytic/point-splat (donut.c style: sample parametric surface, splat with z-test), (b) mini-OpenGL pipelines (TermGL, rendersloth: vertex transform → triangle raster → per-cell shading char/color). Shading char ramps + per-cell z-buffer are universal. HIGH (synthesis).

(further findings below as gathered)
