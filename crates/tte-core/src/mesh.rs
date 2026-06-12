//! Triangle mesh representation (FR-1.2).

use crate::math::Vec3;

/// An indexed triangle mesh.
///
/// Geometry lives in flat arrays (BufferGeometry-style, see project brief D3):
/// `triangles` holds 0-based indices into `positions`.
#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    /// Per-vertex normals; same length as `positions` (derived from face
    /// geometry by the OBJ loader when the file has no `vn` records).
    pub normals: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}

impl Mesh {
    /// Unique undirected edges of the triangle mesh, each as `(lo, hi)` index
    /// pairs, sorted — deterministic output for deterministic frames (NFR-1).
    pub fn edges(&self) -> Vec<(u32, u32)> {
        let mut edges: Vec<(u32, u32)> = self
            .triangles
            .iter()
            .flat_map(|&[a, b, c]| [(a, b), (b, c), (c, a)])
            .map(|(a, b)| (a.min(b), a.max(b)))
            .collect();
        edges.sort_unstable();
        edges.dedup();
        edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr1_2_edges_are_deduplicated_and_sorted() {
        // Two triangles sharing the edge (0,2): 6 directed edges, 5 unique.
        let mesh = Mesh {
            positions: vec![Vec3::ZERO; 4],
            normals: vec![Vec3::Z; 4],
            triangles: vec![[0, 1, 2], [2, 3, 0]],
        };
        assert_eq!(mesh.edges(), vec![(0, 1), (0, 2), (0, 3), (1, 2), (2, 3)]);
    }
}
