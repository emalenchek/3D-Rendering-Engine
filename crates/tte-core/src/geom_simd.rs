//! FR-7.2 SIMD geometry kernel: the per-triangle projection + shading stage,
//! vectorized with `wide::f32x8` — **8 triangles per lane**.
//!
//! This is the profile-confirmed hot stage (docs/research/11b-profile-results.md):
//! perspective divides, the face-normal normalize, Lambert shading, and the
//! viewport map. It is **byte-identical** to the scalar path in
//! [`crate::solid`]'s `prepare_mesh` (FR-7.4). Every operation mirrors the scalar
//! order and uses only `+ - * /` and `sqrt`/`floor`/`ceil` — **never `mul_add`**
//! — so no FMA contraction can occur and the result matches bit-for-bit across
//! SSE2/AVX2/NEON/wasm-simd128 (W4).
//!
//! The integer `orient2d` rasterizer (`triangle::fill_triangle`) is untouched.

use wide::{CmpGt, CmpLe, CmpLt, f32x8};

use crate::color::Material;
use crate::math::{Mat4, Vec3, Vec4};
use crate::shading::{DirectionalLight, ShadingMode};
use crate::solid::DrawTri;
use crate::triangle::Vertex;

const LANES: usize = 8;

/// SIMD per-vertex transform pass (the first half of the geometry stage),
/// 8 vertices per lane. Produces world-space positions, world-space (unit)
/// normals, and clip-space positions — **byte-identical** to the scalar
/// `transform_point` / `transform_dir` / `view_proj * p.extend(1)` it replaces
/// (FR-7.2). Matrix elements are broadcast across lanes; the multiply-add chains
/// replay the scalar `Mat4 * Vec4` dot order (`((m0·x + m1·y) + m2·z) + m3·w`)
/// with no `mul_add`, so no FMA contraction occurs.
pub(crate) fn transform_vertices(
    model: Mat4,
    view_proj: Mat4,
    positions: &[Vec3],
    normals: &[Vec3],
) -> (Vec<Vec3>, Vec<Vec3>, Vec<Vec4>) {
    let n = positions.len();
    debug_assert_eq!(
        n,
        normals.len(),
        "positions and normals are parallel arrays"
    );
    let mut world_pos = Vec::with_capacity(n);
    let mut world_nrm = Vec::with_capacity(n);
    let mut clip = Vec::with_capacity(n);

    let m = |r: usize, c: usize| f32x8::splat(model.m[r][c]);
    let vp = |r: usize, c: usize| f32x8::splat(view_proj.m[r][c]);
    let zero = f32x8::splat(0.0);
    let one = f32x8::splat(1.0);
    let eps = f32x8::splat(f32::EPSILON);

    let mut base = 0;
    while base < n {
        let len = (n - base).min(LANES);
        // Gather 8 vertices (tail padded with the last) into SoA lanes.
        let mut px = [0.0f32; LANES];
        let mut py = [0.0f32; LANES];
        let mut pz = [0.0f32; LANES];
        let mut nx = [0.0f32; LANES];
        let mut ny = [0.0f32; LANES];
        let mut nz = [0.0f32; LANES];
        for l in 0..LANES {
            let j = (base + l).min(n - 1);
            let p = positions[j];
            px[l] = p.x;
            py[l] = p.y;
            pz[l] = p.z;
            let q = normals[j];
            nx[l] = q.x;
            ny[l] = q.y;
            nz[l] = q.z;
        }
        let (px, py, pz) = (f32x8::from(px), f32x8::from(py), f32x8::from(pz));
        let (nx, ny, nz) = (f32x8::from(nx), f32x8::from(ny), f32x8::from(nz));

        // world position = (model * p.extend(1)).truncate()  (w = 1)
        let wx = ((m(0, 0) * px + m(0, 1) * py) + m(0, 2) * pz) + m(0, 3) * one;
        let wy = ((m(1, 0) * px + m(1, 1) * py) + m(1, 2) * pz) + m(1, 3) * one;
        let wz = ((m(2, 0) * px + m(2, 1) * py) + m(2, 2) * pz) + m(2, 3) * one;

        // world normal = (model * n.extend(0)).truncate().normalize().unwrap_or(n).
        // The `* zero` 4th term mirrors the scalar dot exactly (w = 0).
        let tx = ((m(0, 0) * nx + m(0, 1) * ny) + m(0, 2) * nz) + m(0, 3) * zero;
        let ty = ((m(1, 0) * nx + m(1, 1) * ny) + m(1, 2) * nz) + m(1, 3) * zero;
        let tz = ((m(2, 0) * nx + m(2, 1) * ny) + m(2, 2) * nz) + m(2, 3) * zero;
        let nlen = ((tx * tx + ty * ty) + tz * tz).sqrt();
        let nvalid = nlen.cmp_gt(eps);
        let nrecip = one / nlen;
        let onx = nvalid.blend(tx * nrecip, nx);
        let ony = nvalid.blend(ty * nrecip, ny);
        let onz = nvalid.blend(tz * nrecip, nz);

        // clip = view_proj * world_pos.extend(1)  (w = 1)
        let cx = ((vp(0, 0) * wx + vp(0, 1) * wy) + vp(0, 2) * wz) + vp(0, 3) * one;
        let cy = ((vp(1, 0) * wx + vp(1, 1) * wy) + vp(1, 2) * wz) + vp(1, 3) * one;
        let cz = ((vp(2, 0) * wx + vp(2, 1) * wy) + vp(2, 2) * wz) + vp(2, 3) * one;
        let cw = ((vp(3, 0) * wx + vp(3, 1) * wy) + vp(3, 2) * wz) + vp(3, 3) * one;

        let (wx, wy, wz) = (wx.to_array(), wy.to_array(), wz.to_array());
        let (onx, ony, onz) = (onx.to_array(), ony.to_array(), onz.to_array());
        let (cx, cy, cz, cw) = (cx.to_array(), cy.to_array(), cz.to_array(), cw.to_array());
        for l in 0..len {
            world_pos.push(Vec3::new(wx[l], wy[l], wz[l]));
            world_nrm.push(Vec3::new(onx[l], ony[l], onz[l]));
            clip.push(Vec4::new(cx[l], cy[l], cz[l], cw[l]));
        }
        base += LANES;
    }
    (world_pos, world_nrm, clip)
}

/// Build the screen-space triangle list from the per-vertex transform outputs,
/// 8 triangles at a time. Mesh order is preserved (the rasterizer's depth
/// tie-break depends on it), and culled triangles are dropped exactly as the
/// scalar `filter_map` does. Sequential by design: the per-vertex transform
/// passes upstream already parallelize under the `parallel` feature, and keeping
/// this loop scalar-free-of-rayon keeps the byte-identical guarantee obvious.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_draw_tris(
    triangles: &[[u32; 3]],
    clip: &[Vec4],
    world_pos: &[Vec3],
    world_nrm: &[Vec3],
    light: &DirectionalLight,
    shading: ShadingMode,
    material: Material,
    width: u16,
    height: u16,
) -> Vec<DrawTri> {
    let mut out = Vec::with_capacity(triangles.len());

    // Lane-broadcast constants (hoisted out of the batch loop).
    let half = f32x8::splat(0.5);
    let one = f32x8::splat(1.0);
    let zero = f32x8::splat(0.0);
    let eps = f32x8::splat(f32::EPSILON);
    let wm = f32x8::splat(f32::from(width.saturating_sub(1)));
    let hm = f32x8::splat(f32::from(height.saturating_sub(1)));
    let to_light = light.to_light();
    let (lx, ly, lz) = (
        f32x8::splat(to_light.x),
        f32x8::splat(to_light.y),
        f32x8::splat(to_light.z),
    );
    let amb = f32x8::splat(light.ambient);
    let inv_amb = f32x8::splat(1.0 - light.ambient);
    let color = material.base_color;
    let flat = matches!(shading, ShadingMode::Flat);

    // `intensity = clamp(ambient + (1-ambient) * max(0, N·L), 0, 1)` — mirrors
    // DirectionalLight::intensity exactly (dot order, clamp via max/min).
    let intensity = |nx: f32x8, ny: f32x8, nz: f32x8| -> f32x8 {
        let d = (nx * lx + ny * ly) + nz * lz;
        let diffuse = d.max(zero);
        (amb + inv_amb * diffuse).max(zero).min(one)
    };

    // Perspective divide + viewport map (screen x/y), NDC depth — mirrors
    // `solid::screen_vertex`: `inv_w = 1/w` then multiply (not a direct `x/w`).
    let project = |cx: f32x8, cy: f32x8, cz: f32x8, cw: f32x8| {
        let inv_w = one / cw;
        let ndc_x = cx * inv_w;
        let ndc_y = cy * inv_w;
        let sx = (ndc_x * half + half) * wm;
        let sy = (half - ndc_y * half) * hm;
        let depth = cz * inv_w;
        (sx, sy, depth)
    };

    // Flat shading needs each face's three world *positions* (for the face
    // normal); Gouraud needs each vertex's world *normal*. Only one set is
    // gathered, from whichever slice this selects.
    let shade_src = if flat { world_pos } else { world_nrm };

    for chunk in triangles.chunks(LANES) {
        let len = chunk.len();

        // AoSoA transpose: gather this batch's vertex attributes into lane arrays
        // in a single pass — clip x/y/z/w for each of the 3 vertices, plus the
        // shading source (positions or normals). A short tail repeats the last
        // real triangle; padded lanes are never emitted (the `0..len` loop skips
        // them), so their values are inert.
        let mut cax = [0.0f32; LANES];
        let mut cay = [0.0f32; LANES];
        let mut caz = [0.0f32; LANES];
        let mut caw = [0.0f32; LANES];
        let mut cbx = [0.0f32; LANES];
        let mut cby = [0.0f32; LANES];
        let mut cbz = [0.0f32; LANES];
        let mut cbw = [0.0f32; LANES];
        let mut ccx = [0.0f32; LANES];
        let mut ccy = [0.0f32; LANES];
        let mut ccz = [0.0f32; LANES];
        let mut ccw = [0.0f32; LANES];
        let mut pax = [0.0f32; LANES];
        let mut pay = [0.0f32; LANES];
        let mut paz = [0.0f32; LANES];
        let mut pbx = [0.0f32; LANES];
        let mut pby = [0.0f32; LANES];
        let mut pbz = [0.0f32; LANES];
        let mut pcx = [0.0f32; LANES];
        let mut pcy = [0.0f32; LANES];
        let mut pcz = [0.0f32; LANES];
        for l in 0..LANES {
            let [ia, ib, ic] = chunk[l.min(len - 1)];
            let (ia, ib, ic) = (ia as usize, ib as usize, ic as usize);
            let a = clip[ia];
            cax[l] = a.x;
            cay[l] = a.y;
            caz[l] = a.z;
            caw[l] = a.w;
            let b = clip[ib];
            cbx[l] = b.x;
            cby[l] = b.y;
            cbz[l] = b.z;
            cbw[l] = b.w;
            let c = clip[ic];
            ccx[l] = c.x;
            ccy[l] = c.y;
            ccz[l] = c.z;
            ccw[l] = c.w;
            let a = shade_src[ia];
            pax[l] = a.x;
            pay[l] = a.y;
            paz[l] = a.z;
            let b = shade_src[ib];
            pbx[l] = b.x;
            pby[l] = b.y;
            pbz[l] = b.z;
            let c = shade_src[ic];
            pcx[l] = c.x;
            pcy[l] = c.y;
            pcz[l] = c.z;
        }
        let (cax, cay, caz, caw) = (
            f32x8::from(cax),
            f32x8::from(cay),
            f32x8::from(caz),
            f32x8::from(caw),
        );
        let (cbx, cby, cbz, cbw) = (
            f32x8::from(cbx),
            f32x8::from(cby),
            f32x8::from(cbz),
            f32x8::from(cbw),
        );
        let (ccx, ccy, ccz, ccw) = (
            f32x8::from(ccx),
            f32x8::from(ccy),
            f32x8::from(ccz),
            f32x8::from(ccw),
        );
        let (pax, pay, paz) = (f32x8::from(pax), f32x8::from(pay), f32x8::from(paz));
        let (pbx, pby, pbz) = (f32x8::from(pbx), f32x8::from(pby), f32x8::from(pbz));
        let (pcx, pcy, pcz) = (f32x8::from(pcx), f32x8::from(pcy), f32x8::from(pcz));

        // Near-plane cull (FR-1.3): drop a triangle if any vertex is behind the
        // near plane — `w <= 0 || z < -w` — matching `solid::behind_near`.
        let behind = |cz: f32x8, cw: f32x8| cw.cmp_le(zero) | cz.cmp_lt(zero - cw);
        let cull = behind(caz, caw) | behind(cbz, cbw) | behind(ccz, ccw);
        let cull_mask = cull.move_mask();

        // Project all three vertices.
        let (sxa, sya, dpa) = project(cax, cay, caz, caw);
        let (sxb, syb, dpb) = project(cbx, cby, cbz, cbw);
        let (sxc, syc, dpc) = project(ccx, ccy, ccz, ccw);

        // Per-vertex shading intensities.
        let (ia, ib, ic) = if flat {
            // Flat: one face normal for all three vertices. `n = normalize(
            // (b-a) × (c-a))`, with the scalar's `Vec3::Z` fallback for a
            // degenerate (zero-area) face. pa/pb/pc are world positions.
            let (e1x, e1y, e1z) = (pbx - pax, pby - pay, pbz - paz);
            let (e2x, e2y, e2z) = (pcx - pax, pcy - pay, pcz - paz);
            let crx = e1y * e2z - e1z * e2y;
            let cry = e1z * e2x - e1x * e2z;
            let crz = e1x * e2y - e1y * e2x;
            // normalize: len = sqrt((x²+y²)+z²); n = cross * (1/len) if len > eps.
            let len = ((crx * crx + cry * cry) + crz * crz).sqrt();
            let valid = len.cmp_gt(eps);
            let rlen = one / len;
            let fnx = valid.blend(crx * rlen, zero);
            let fny = valid.blend(cry * rlen, zero);
            let fnz = valid.blend(crz * rlen, one);
            let fi = intensity(fnx, fny, fnz);
            (fi, fi, fi)
        } else {
            // Gouraud: pa/pb/pc are the per-vertex world normals.
            (
                intensity(pax, pay, paz),
                intensity(pbx, pby, pbz),
                intensity(pcx, pcy, pcz),
            )
        };

        // Row span: min floor / max ceil of the three screen y's (matches the
        // scalar `p.y.floor()/ceil() as i32`; float min/max of integer-valued
        // floors equals the integer min/max after the cast).
        let y_min = sya.floor().min(syb.floor()).min(syc.floor()).to_array();
        let y_max = sya.ceil().max(syb.ceil()).max(syc.ceil()).to_array();

        let (sxa, sya, dpa, ia) = (
            sxa.to_array(),
            sya.to_array(),
            dpa.to_array(),
            ia.to_array(),
        );
        let (sxb, syb, dpb, ib) = (
            sxb.to_array(),
            syb.to_array(),
            dpb.to_array(),
            ib.to_array(),
        );
        let (sxc, syc, dpc, ic) = (
            sxc.to_array(),
            syc.to_array(),
            dpc.to_array(),
            ic.to_array(),
        );

        for l in 0..len {
            if cull_mask & (1 << l) != 0 {
                continue; // a vertex was behind the near plane
            }
            out.push(DrawTri {
                v: [
                    Vertex {
                        x: sxa[l],
                        y: sya[l],
                        depth: dpa[l],
                        intensity: ia[l],
                    },
                    Vertex {
                        x: sxb[l],
                        y: syb[l],
                        depth: dpb[l],
                        intensity: ib[l],
                    },
                    Vertex {
                        x: sxc[l],
                        y: syc[l],
                        depth: dpc[l],
                        intensity: ic[l],
                    },
                ],
                color,
                y_min: y_min[l] as i32,
                y_max: y_max[l] as i32,
            });
        }
    }
    out
}
