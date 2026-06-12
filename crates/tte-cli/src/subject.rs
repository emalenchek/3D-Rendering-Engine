//! Loading the thing a `view` displays: a single OBJ mesh, or a DSL scene with
//! its referenced mesh assets (FR-4.6).

use std::collections::HashMap;
use std::path::Path;
use tte_core::{Mesh, Scene};

/// What `tte view` is showing.
#[derive(Debug)]
pub enum Subject {
    /// A single OBJ model (Phase 1–3 behaviour: a spinning object).
    Mesh(Mesh),
    /// A DSL scene plus its preloaded external meshes, keyed by `src` path.
    Scene {
        scene: Scene,
        assets: HashMap<String, Mesh>,
    },
}

/// Load a `view` target by file extension: `.obj` → mesh, `.scene`/`.kdl` → DSL
/// scene. Scene mesh references are resolved (and preloaded) relative to the
/// scene file's directory; a missing asset is a hard error.
pub fn load(path: &Path) -> Result<Subject, Box<dyn std::error::Error>> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("obj") => Ok(Subject::Mesh(tte_core::load_obj(path)?)),
        Some("scene") | Some("kdl") => {
            let text = std::fs::read_to_string(path)
                .map_err(|e| format!("could not open scene '{}': {e}", path.display()))?;
            let scene = tte_core::parse_scene(&text)?;
            let base = path.parent().unwrap_or_else(|| Path::new("."));
            let mut assets = HashMap::new();
            for src in scene.mesh_refs() {
                if !assets.contains_key(src) {
                    let mesh = tte_core::load_obj(&base.join(src))
                        .map_err(|e| format!("scene references mesh '{src}': {e}"))?;
                    assets.insert(src.to_string(), mesh);
                }
            }
            Ok(Subject::Scene { scene, assets })
        }
        _ => Err(format!(
            "unsupported file type '{}' — use a .obj model or a .scene file",
            path.display()
        )
        .into()),
    }
}
