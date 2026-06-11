# Research: ASCII/Text 3D Rendering in Terminals — Prior Art & Performance Limits

Date: 2026-06-11. Each claim carries source URL(s) and confidence HIGH/MEDIUM/LOW.

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

## Q3. Sub-cell resolution (half-blocks, quadrants, sextants, octants, braille)

- Notcurses "blitter" taxonomy (canonical reference for sub-cell modes), from notcurses_visual(3) man page (https://github.com/dankamongmen/notcurses/blob/master/doc/man/man3/notcurses_visual.3.md, mirrored at https://notcurses.com/notcurses_visual.3.html). Confidence HIGH (fetched raw man source):
  - NCBLIT_1x1: 1 subdivision/cell, space glyph, bg color only — the plain "one cell = one pixel" mode; works even in pure ASCII.
  - NCBLIT_2x1: half blocks (▀▄), 2 vertical subdivisions/cell — doubles vertical resolution AND preserves the ~1:2 cell aspect ratio, so pixels come out square. Each sub-pixel gets its own *full color* (one via fg, one via bg). This is the sweet spot for color images.
  - NCBLIT_2x2: quadrants (▌▐▖▗▟▙ etc.), 2x2 = 4 subdivisions/cell — but only 2 colors per cell (fg+bg), so 4 sub-pixels must be quantized to 2 colors.
  - NCBLIT_3x2: sextants (Unicode 13 "Symbols for Legacy Computing"), 3 rows x 2 cols = 6 subdivisions/cell; stretches image 1.5x vertically vs 2x2; still 2 colors/cell.
  - NCBLIT_4x2: octants (Unicode 16) + quadrants, 4x2 = 8 subdivisions/cell; 2 colors/cell — "can lose color fidelity" on complex blocks.
  - NCBLIT_BRAILLE: braille patterns U+2800–U+28FF, 4 rows x 2 cols = 8 dots/cell, 256 distinct glyphs; effectively *binary* per dot with one fg color per cell — the man page notes it "doesn't tend to work out very well for images" (best for monochrome line art / wireframes / plots). U+2800 blank gives consistent spacing; braille has wide font support and fixed advance width even in proportional fonts. Sources: man page above + https://news.ycombinator.com/item?id=24956014 — HIGH.
  - NCBLIT_PIXEL: true bitmap via Sixel/Kitty when available. Degradation order when glyphs unsupported: 4x2 → 3x2 → 2x2 → 2x1 → 1x1. HIGH.
- **Key universal constraint: a terminal cell carries exactly ONE foreground + ONE background color.** All sub-cell schemes beyond 2x1 trade chroma resolution for spatial resolution (like chroma subsampling). Half-blocks (2x1) are the only mode where every sub-pixel keeps independent full color. HIGH (follows directly from the man page table; corroborated by chafa/notcurses design).
- Resolution math at a 200x50-cell terminal: 1x1 → 200x50 px; 2x1 half-blocks → 200x100; 2x2 quadrants → 400x100; 3x2 sextants → 400x150; 4x2 octants/braille → 400x200 (braille: binary dots only). Synthesis — HIGH (arithmetic).
- Caveat: sextants (U+13) and octants (U+16) have patchier font/terminal support than half-blocks/quadrants/braille; notcurses auto-degrades for this reason. MEDIUM.

## Q4. Terminal graphics libraries as conceptual prior art

- **chafa** (https://github.com/hpjansson/chafa): C library + CLI converting images/GIF animations to terminal output; supports sixel, Kitty protocol, iTerm2 protocol, and ANSI/Unicode character art; targets "devices ranging from historical teleprinters to modern terminal emulators"; LGPLv3+, ~92% C. Picks the best glyph+fg+bg per cell from configurable symbol classes (blocks/quadrants/sextants/braille/ASCII...) by error minimization; supports 2/8/16/256/truecolor modes and dithering (per docs/blog; blog posts at hpjansson.org returned 403 — symbol-class details from search snippets and chafa man page knowledge). HIGH for feature list on repo page; MEDIUM for per-cell algorithm details.
- **notcurses** (https://github.com/dankamongmen/notcurses): TUI/"character graphics" library by Nick Black; plane-based compositor, the blitters above, multimedia decoding, and a heavily optimized rasterizer that diffs cell state and emits minimal escape sequences. HIGH.
- **libcaca**: classic color-ASCII art library (successor to aalib); dithering of color images onto 16-color fg/bg + character density; conceptual ancestor of chafa. MEDIUM (well-known, not fetched this session).
- **timg / viu**: terminal image viewers using half-block (2x1) truecolor cell art with fallback, plus Sixel/Kitty/iTerm2 where available — evidence that half-block truecolor is the de-facto portable "pixel" mode. MEDIUM (not fetched; widely documented).
- Common pipeline in all of these: scale image to cell-grid-derived resolution → (optional dither) → per-cell glyph+color selection → color quantization to terminal palette → escape-sequence emission with SGR batching. Synthesis — HIGH.

## Q5. Terminal performance limits

- **Raw throughput is no longer the bottleneck on modern terminals.** 2026 comparisons report Ghostty ~3x faster than iTerm2 and ~2.5x faster than Warp on raw throughput; Alacritty/Kitty/Ghostty/WezTerm are all in the "fast" GPU-accelerated class. Key-to-screen latency: Ghostty ~2ms, Alacritty/Kitty ~3ms, Warp ~8ms, iTerm2 ~12ms. Sources: https://scopir.com/posts/best-terminal-emulators-developers-2026/ , https://dasroot.net/posts/2026/03/linux-terminal-emulators-alacritty-kitty-wezterm/ , https://sw.kovidgoyal.net/kitty/performance/ — MEDIUM (blog benchmarks, methodology varies).
- **Worst-case bandwidth at 200x50 truecolor (computed):** 10,000 cells x ~40 bytes (truecolor fg `ESC[38;2;R;G;Bm` ~19B + bg `ESC[48;2;R;G;Bm` ~19B + 1 char, amortized cursor moves) ≈ 400 KB/frame → ~24 MB/s at 60 FPS. Fast terminals parse input at tens-to-hundreds of MB/s, so 30–60 FPS full-truecolor redraw is feasible on Alacritty/Kitty/Ghostty/WezTerm but marginal-to-bad on slow terminals (older VTE, Terminal.app, anything over SSH). Synthesis (arithmetic + benchmark class above) — MEDIUM.
- **Diff-based redraw is the single biggest win:** keep a front buffer of last-emitted cells, emit only changed cells, coalesce runs, skip SGR re-emission when fg/bg/attrs unchanged (batch SGR changes). This is exactly what notcurses' rasterizer and every TUI framework (ncurses, bubbletea) does; for 3D scenes (object on stable background) it cuts bytes by 5–50x. Sources: https://github.com/dankamongmen/notcurses (rasterizer design), https://github.com/dankamongmen/notcurses/discussions/2157 — HIGH (well-established technique), savings figure MEDIUM (scene-dependent).
- **Synchronized output (DEC private mode 2026, `CSI ? 2026 h/l`):** wraps a frame so the terminal renders it atomically — eliminates tearing/flicker and reportedly improves effective performance 20–50% on supporting terminals; ~14+ terminals support it in 2026 (incl. kitty, WezTerm, Ghostty, foot, Contour, recent VTE, Windows Terminal). Must be capability-queried (DECRQM) with a timeout — naive querying leaks escape codes in short-lived processes (see bubbletea v2 bug). Sources: https://terminaltrove.com/compare/terminals/ , https://github.com/charmbracelet/bubbletea/issues/1627 — HIGH for existence/purpose, MEDIUM for the 20–50% figure.
- Other practical limits: stdout should be block-buffered with one `write()` per frame (PTY writes are chunked ~64KB; many small writes kill throughput); cap FPS to terminal refresh (60Hz) and to what the scene needs (24–30 is fine, cf. ASCILINE); `printf`-per-cell is the classic perf mistake. Synthesis — HIGH (standard systems knowledge).
- **Realistic verdict for ~200x50 truecolor:** 60 FPS achievable on fast terminals with diff-redraw + SGR batching + single buffered write + mode 2026; design budget should assume ~30 FPS portable floor. Synthesis — MEDIUM.

## Q6. Pixel protocols (future option, brief)

- Three protocols: **Sixel** (DEC legacy, paletted, widest support: xterm, iTerm2 ≥3.3, Konsole, foot, WezTerm, Windows Terminal ≥1.23, mlterm; NOT kitty, NOT default GNOME-Terminal/VTE builds, NOT tmux passthrough), **Kitty graphics protocol** (modern, RGBA, compression, placements/animation; kitty, Ghostty, WezTerm, Konsole), **iTerm2 inline images** (base64 PNG; iTerm2, WezTerm, VS Code xterm.js, mintty). Sources: https://akmatori.com/blog/terminal-graphics-protocols , https://tmuxai.dev/terminal-compatibility/ , https://github.com/BourgeoisBear/rasterm — HIGH on the broad matrix, MEDIUM on per-version details.
- Alacritty supports none of the image protocols (by design). Same sources — HIGH.
- chafa and notcurses (NCBLIT_PIXEL) already abstract over all three with cell-art fallback — the proven architecture: detect capability, prefer pixels, degrade to cells. Sources: https://github.com/hpjansson/chafa , notcurses_visual(3) — HIGH.
- Trend: Kitty protocol adoption is growing (Ghostty, WezTerm, Konsole); Sixel remains the lowest common denominator fallback. Source: https://akmatori.com/blog/terminal-graphics-protocols — MEDIUM.

## Synthesis per question (one-liners)

1. **ASCILINE** is a video→ASCII streamer (Python/OpenCV/WebSocket/Canvas), not a 3D engine; borrow its vectorized pixel→char mapping, tiered color modes, and pacing — the 3D pipeline is ours to build.
2. **Prior 3D art** splits into point-splat (donut.c: parametric sampling + per-cell z-buffer + 12-char luminance ramp) and mini-GL pipelines (TermGL, rendersloth: vertex transform → triangle raster → per-cell char+color). Per-cell z-buffer and luminance ramps are universal.
3. **Sub-cell glyphs** multiply resolution up to 8x (octants/braille 4x2) but every cell still has only one fg+bg color; half-blocks (2x1) are the only mode with full color per sub-pixel; braille is binary/mono — ideal for wireframes.
4. **chafa/notcurses/libcaca/timg/viu** establish the standard pipeline: scale → dither → per-cell glyph+color selection → palette quantization → minimal escape emission, with graceful degradation across terminal capabilities.
5. **Performance:** modern GPU terminals make 200x50@60FPS truecolor feasible; the engine-side requirements are diff-redraw, SGR batching, one buffered write/frame, and mode-2026 sync; assume 30 FPS portable floor.
6. **Pixel protocols** (Sixel/Kitty/iTerm2) are a capability-gated future output backend, already proven viable behind the chafa/notcurses abstraction pattern.

## Implications for our MVP

**Adopt for v1:**
- Software rasterizer over a cell grid with **per-cell z-buffer** and **luminance→character ramp** (donut.c's `.,-~:;=!*#$@` as default) — the proven core of every terminal 3D renderer.
- Triangle pipeline (TermGL/rendersloth model) rather than donut-style point splatting: generalizes to arbitrary meshes; treat each cell (or sub-cell) as a pixel.
- **Half-block (2x1) truecolor mode** as the "high fidelity" default where supported: 2x vertical resolution, square pixels, full color per sub-pixel, near-universal glyph support. Plain ASCII ramp as the universal fallback.
- **Braille (4x2) mode for wireframe/mono rendering** — 8x dot resolution, perfect for line art; cheap to add alongside.
- Renderer back end: front/back cell buffers with **diff-based redraw**, SGR change batching, single buffered write per frame, optional **DEC mode 2026** sync (capability-detect with timeout), FPS cap (default 30, allow 60).
- Tiered color: mono → 16 → 256 → truecolor, selected by capability detection (TERM/COLORTERM + queries).

**Defer post-v1:**
- Quadrant/sextant/octant blitters with 2-color-per-cell quantization (chafa-style error minimization) — meaningful complexity for modest gains over half-blocks.
- Sixel/Kitty/iTerm2 pixel backends — design the output layer as a trait/interface now so a pixel backend can slot in later; don't implement yet.
- Dithering and perceptual color quantization (libcaca/chafa-grade) — only matters once textured/shaded scenes exceed what truecolor half-blocks show cleanly.
- Web/Canvas frontend (ASCILINE-style WebSocket streaming) — interesting distribution channel, out of scope for a terminal engine MVP.
