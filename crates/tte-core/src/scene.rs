//! Scene model: the semantic form of a parsed DSL document (FR-4.2).
//!
//! Mirrors the DSL closely (so it can round-trip: parse → `Scene` → serialize →
//! parse yields the same `Scene`, FR-4.7) and flattens to a renderable draw list
//! (FR-4.5). Geometry is described, not baked, until render time.

use crate::camera::Camera;
use crate::color::{Material, Rgb};
use crate::math::{Mat4, Vec3};
use crate::shading::DirectionalLight;

/// A complete scene: lighting/background plus a tree of placed objects.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub background: Rgb,
    pub camera: Option<SceneCamera>,
    pub light: SceneLight,
    /// Named, reusable materials (define-once / use-many; insertion order kept).
    pub materials: Vec<(String, Rgb)>,
    pub roots: Vec<Node>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            background: Rgb::BLACK,
            camera: None,
            light: SceneLight::default(),
            materials: Vec::new(),
            roots: Vec::new(),
        }
    }
}

/// Camera as written in the DSL (`position`, `look-at`, `fov` in degrees).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneCamera {
    pub position: Vec3,
    pub look_at: Vec3,
    pub fov_deg: f32,
}

impl SceneCamera {
    /// Convert to a renderable [`Camera`] (keeps the engine's default lens
    /// near/far and cell aspect).
    pub fn to_camera(&self) -> Camera {
        Camera {
            eye: self.position,
            target: self.look_at,
            up: Vec3::Y,
            fov_y: self.fov_deg.to_radians(),
            ..Camera::default()
        }
    }
}

/// Directional light as written in the DSL.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneLight {
    pub direction: Vec3,
    pub intensity: f32,
    pub ambient: f32,
}

impl Default for SceneLight {
    fn default() -> Self {
        Self { direction: Vec3::new(-0.5, -1.0, -0.8), intensity: 1.0, ambient: 0.15 }
    }
}

impl SceneLight {
    pub fn to_light(&self) -> DirectionalLight {
        DirectionalLight::new(self.direction, self.ambient)
    }
}

/// A placed node: a local transform, optional geometry, optional material
/// reference, and children (whose transforms compose with this one).
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub name: Option<String>,
    pub transform: Transform,
    pub geometry: Geometry,
    /// Material name referencing [`Scene::materials`]; `None` → engine default.
    pub material: Option<String>,
    pub children: Vec<Node>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            name: None,
            transform: Transform::default(),
            geometry: Geometry::Group,
            material: None,
            children: Vec::new(),
        }
    }
}

/// What a node draws.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    /// Transform-only group (no geometry of its own).
    Group,
    Cube,
    Sphere { rings: u32, segments: u32 },
    Plane,
    /// External mesh asset (OBJ), resolved relative to the scene file.
    MeshRef(String),
}

/// Local TRS transform. Applied as translate · rotate(Z·Y·X) · scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translate: Vec3,
    /// Euler angles in degrees (applied X, then Y, then Z).
    pub rotate_deg: Vec3,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translate: Vec3::ZERO,
            rotate_deg: Vec3::ZERO,
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        let r = self.rotate_deg;
        let rot = Mat4::rotation_z(r.z.to_radians())
            * Mat4::rotation_y(r.y.to_radians())
            * Mat4::rotation_x(r.x.to_radians());
        Mat4::translation(self.translate) * rot * Mat4::scale(self.scale)
    }
}

/// One flattened drawable: world transform, the geometry to bake, and the
/// resolved surface material.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawable {
    pub world: Mat4,
    pub geometry: Geometry,
    pub material: Material,
}

impl Scene {
    /// Look up a named material's color, falling back to the engine default.
    pub fn resolve_material(&self, name: Option<&str>) -> Material {
        match name {
            Some(n) => self
                .materials
                .iter()
                .find(|(k, _)| k == n)
                .map(|(_, c)| Material { base_color: *c })
                .unwrap_or_default(),
            None => Material::default(),
        }
    }

    /// Flatten the node tree into world-space drawables (FR-4.5). Group nodes
    /// contribute only their transform to descendants.
    pub fn flatten(&self) -> Vec<Drawable> {
        let mut out = Vec::new();
        for node in &self.roots {
            self.flatten_node(node, Mat4::IDENTITY, &mut out);
        }
        out
    }

    fn flatten_node(&self, node: &Node, parent: Mat4, out: &mut Vec<Drawable>) {
        let world = parent * node.transform.matrix();
        if node.geometry != Geometry::Group {
            out.push(Drawable {
                world,
                geometry: node.geometry.clone(),
                material: self.resolve_material(node.material.as_deref()),
            });
        }
        for child in &node.children {
            self.flatten_node(child, world, out);
        }
    }

    /// Names of every external mesh referenced (for asset preloading/validation).
    pub fn mesh_refs(&self) -> Vec<&str> {
        fn walk<'a>(node: &'a Node, acc: &mut Vec<&'a str>) {
            if let Geometry::MeshRef(path) = &node.geometry {
                acc.push(path);
            }
            node.children.iter().for_each(|c| walk(c, acc));
        }
        let mut acc = Vec::new();
        self.roots.iter().for_each(|n| walk(n, &mut acc));
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn fr4_5_flatten_composes_child_transforms() {
        let child = Node {
            transform: Transform { translate: Vec3::new(1.0, 0.0, 0.0), ..Default::default() },
            geometry: Geometry::Cube,
            ..Default::default()
        };
        let parent = Node {
            transform: Transform { translate: Vec3::new(0.0, 2.0, 0.0), ..Default::default() },
            children: vec![child],
            ..Default::default()
        };
        let scene = Scene { roots: vec![parent], ..Default::default() };
        let drawables = scene.flatten();
        assert_eq!(drawables.len(), 1, "group parent contributes no geometry");
        // Cube ends up at parent+child translation = (1, 2, 0).
        let origin = (drawables[0].world * Vec3::ZERO.extend(1.0)).truncate();
        assert_abs_diff_eq!(origin, Vec3::new(1.0, 2.0, 0.0), epsilon = 1e-5);
    }

    #[test]
    fn fr4_3_resolve_material_falls_back_to_default() {
        let scene = Scene {
            materials: vec![("red".into(), Rgb::new(200, 10, 10))],
            ..Default::default()
        };
        assert_eq!(scene.resolve_material(Some("red")).base_color, Rgb::new(200, 10, 10));
        assert_eq!(scene.resolve_material(Some("missing")), Material::default());
        assert_eq!(scene.resolve_material(None), Material::default());
    }
}
