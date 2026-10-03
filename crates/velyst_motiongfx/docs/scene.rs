pub use bevy::prelude::*;
pub use bevy_motiongfx::prelude::*;
pub use velyst_motiongfx::prelude::*;

// Matches `#let scene() = ...` in `scene.typ`.
typst_func!(
    "scene",
    #[derive(Default, Clone)]
    pub struct SceneFunc {}
);
