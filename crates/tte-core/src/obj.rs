//! Wavefront OBJ loader for the minimal subset (FR-1.2).
//!
//! Supported records: `v` (position), `vn` (normal), `vt` (parsed, currently
//! unused), `f` (faces with all four index forms: `v`, `v/vt`, `v//vn`,
//! `v/vt/vn`; 1-based and negative/relative indices; >3-gons fan-triangulated).
//! Unknown line types are ignored per the OBJ tradition. Missing normals are
//! derived from face geometry. Scope rationale: docs/research/05-mvp-scoping.md §6.

use crate::math::Vec3;
use crate::mesh::Mesh;

/// Parse failure, with the 1-based source line for friendly diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ObjError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OBJ parse error at line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ObjError {}

/// Parse OBJ text into a [`Mesh`].
pub fn parse_obj(source: &str) -> Result<Mesh, ObjError> {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut file_normals: Vec<Vec3> = Vec::new();
    // (position index, normal index if the file gave one), per face corner.
    let mut triangles: Vec<[(u32, Option<u32>); 3]> = Vec::new();

    for (line_no, raw_line) in source.lines().enumerate() {
        let line_no = line_no + 1;
        let line = raw_line.split('#').next().unwrap_or("").trim();
        let mut tokens = line.split_whitespace();
        match tokens.next() {
            Some("v") => positions.push(parse_vec3(&mut tokens, line_no, "v")?),
            Some("vn") => file_normals.push(parse_vec3(&mut tokens, line_no, "vn")?),
            Some("vt") => { /* texcoords parsed-and-ignored until texturing phase */ }
            Some("f") => {
                let corners: Vec<(u32, Option<u32>)> = tokens
                    .map(|t| parse_face_corner(t, positions.len(), file_normals.len(), line_no))
                    .collect::<Result<_, _>>()?;
                if corners.len() < 3 {
                    return Err(err(line_no, "face needs at least 3 vertices"));
                }
                // Fan triangulation: correct for convex polygons (MVP scope).
                for i in 1..corners.len() - 1 {
                    triangles.push([corners[0], corners[i], corners[i + 1]]);
                }
            }
            // Ignore-what-you-don't-know: o/g/s/usemtl/mtllib/l/p, blanks, etc.
            _ => {}
        }
    }

    let normals = resolve_normals(&positions, &file_normals, &triangles);
    Ok(Mesh {
        positions,
        normals,
        triangles: triangles.iter().map(|t| t.map(|(p, _)| p)).collect(),
    })
}

/// Read a mesh from an OBJ file on disk.
pub fn load_obj(path: &std::path::Path) -> Result<Mesh, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("could not open scene '{}': {e}", path.display()))?;
    Ok(parse_obj(&text)?)
}

fn err(line: usize, message: impl Into<String>) -> ObjError {
    ObjError {
        line,
        message: message.into(),
    }
}

fn parse_vec3<'a>(
    tokens: &mut impl Iterator<Item = &'a str>,
    line: usize,
    record: &str,
) -> Result<Vec3, ObjError> {
    let mut next_float = |axis: &str| {
        tokens
            .next()
            .ok_or_else(|| err(line, format!("{record}: missing {axis} component")))?
            .parse::<f32>()
            .map_err(|_| err(line, format!("{record}: invalid {axis} component")))
    };
    Ok(Vec3::new(
        next_float("x")?,
        next_float("y")?,
        next_float("z")?,
    ))
}

/// Parse one face corner (`v`, `v/vt`, `v//vn`, or `v/vt/vn`), resolving
/// 1-based and negative (relative-to-end) indices to 0-based.
fn parse_face_corner(
    token: &str,
    position_count: usize,
    normal_count: usize,
    line: usize,
) -> Result<(u32, Option<u32>), ObjError> {
    let mut parts = token.split('/');
    let pos = resolve_index(parts.next().unwrap_or(""), position_count, line, "vertex")?
        .ok_or_else(|| err(line, format!("face corner '{token}' has no vertex index")))?;
    let _texcoord = parts.next(); // vt index: accepted, unused for now
    let normal = match parts.next() {
        Some(t) => resolve_index(t, normal_count, line, "normal")?,
        None => None,
    };
    Ok((pos, normal))
}

/// OBJ index → 0-based: positive is 1-based, negative counts back from the end
/// of the array *as parsed so far* (part of the spec). Empty string → None.
fn resolve_index(
    token: &str,
    len: usize,
    line: usize,
    what: &str,
) -> Result<Option<u32>, ObjError> {
    if token.is_empty() {
        return Ok(None);
    }
    let idx: i64 = token
        .parse()
        .map_err(|_| err(line, format!("invalid {what} index '{token}'")))?;
    let resolved = if idx > 0 { idx - 1 } else { len as i64 + idx };
    if idx == 0 || resolved < 0 || resolved >= len as i64 {
        return Err(err(
            line,
            format!("{what} index {idx} out of range (have {len})"),
        ));
    }
    Ok(Some(resolved as u32))
}

/// Use file normals where every corner referencing a vertex agrees; otherwise
/// derive smooth per-vertex normals by averaging adjacent face normals.
fn resolve_normals(
    positions: &[Vec3],
    file_normals: &[Vec3],
    triangles: &[[(u32, Option<u32>); 3]],
) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    let mut from_file = vec![false; positions.len()];

    for tri in triangles {
        let face_normal = {
            let [a, b, c] = tri.map(|(p, _)| positions[p as usize]);
            (b - a).cross(c - a)
        };
        for &(p, n) in tri {
            match n {
                Some(n) => {
                    normals[p as usize] = file_normals[n as usize];
                    from_file[p as usize] = true;
                }
                // Area-weighted accumulation (cross product length ∝ area).
                None if !from_file[p as usize] => {
                    normals[p as usize] = normals[p as usize] + face_normal;
                }
                None => {}
            }
        }
    }

    for n in &mut normals {
        *n = n.normalize().unwrap_or(Vec3::Z);
    }
    normals
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn fr1_2_parses_positions_and_triangle() {
        let mesh = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        assert_eq!(mesh.positions.len(), 3);
        assert_eq!(mesh.triangles, vec![[0, 1, 2]]);
    }

    #[test]
    fn fr1_2_supports_all_four_face_index_forms() {
        let body = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvn 0 0 1\n";
        for face in [
            "f 1 2 3",
            "f 1/1 2/1 3/1",
            "f 1//1 2//1 3//1",
            "f 1/1/1 2/1/1 3/1/1",
        ] {
            let mesh = parse_obj(&format!("{body}{face}\n")).unwrap();
            assert_eq!(mesh.triangles, vec![[0, 1, 2]], "form: {face}");
        }
    }

    #[test]
    fn fr1_2_negative_indices_resolve_from_end() {
        let mesh = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n").unwrap();
        assert_eq!(mesh.triangles, vec![[0, 1, 2]]);
    }

    #[test]
    fn fr1_2_quad_faces_fan_triangulate() {
        let mesh = parse_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n").unwrap();
        assert_eq!(mesh.triangles, vec![[0, 1, 2], [0, 2, 3]]);
    }

    #[test]
    fn fr1_2_unknown_lines_and_comments_are_ignored() {
        let src = "# comment\nmtllib x.mtl\no thing\ns off\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        assert_eq!(parse_obj(src).unwrap().triangles.len(), 1);
    }

    #[test]
    fn fr1_2_missing_normals_are_derived_from_faces() {
        let mesh = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        // CCW triangle in the XY plane → +Z normal.
        for n in &mesh.normals {
            assert_relative_eq!(*n, Vec3::Z, epsilon = 1e-6);
        }
    }

    #[test]
    fn fr1_2_out_of_range_index_errors_with_line_number() {
        let e = parse_obj("v 0 0 0\nf 1 2 3\n").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(e.message.contains("out of range"), "got: {}", e.message);
    }

    #[test]
    fn fr1_2_malformed_float_errors_with_line_number() {
        let e = parse_obj("v 0 zero 0\n").unwrap_err();
        assert_eq!(e.line, 1);
    }
}
