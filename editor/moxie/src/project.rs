//! Saving and loading the editor's project file.
//!
//! A `.mox` holds both halves of a project in one RON document: the
//! entities, as a reflected [`DynamicWorld`], and the animation over
//! them, as a [`Scene`]. Neither is much use alone: the animation
//! addresses its subjects by [`EntityUid`], which only means anything
//! once the entities carrying those ids are back in the world.

use std::path::{Path, PathBuf};

use bevy::asset::{AssetServer, LoadFromPath};
use bevy::light::CascadeShadowConfig;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use bevy::world_serialization::serde::{
    DynamicWorldSerializer, WorldDeserializer,
};
use bevy::world_serialization::{
    DynamicWorld, DynamicWorldBuilder, WorldFilter,
};
use bevy_motiongfx::scene::asset::MotionGfxScene;
use bevy_motiongfx::scene::backend::Backend;
use bevy_motiongfx::scene::id::EntityUid;
use motiongfx_scene::scene::Scene;
use serde::de::{DeserializeSeed, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserializer, Serialize, Serializer};

use crate::{
    EditorScene, ProjectBookmarks, ProjectPath, SceneRoot,
    SelectedAction, SelectedEntity,
};

const EXTENSION: &str = "mox";

// The name a project file is written and read under, and its fields.
// Shared so the reader and the writer cannot drift apart.
const PROJECT: &str = "Project";
const WORLD: &str = "world";
const SCENE: &str = "scene";
const BOOKMARKS: &str = "bookmarks";

/// Prompts for a path and writes the whole project to it.
pub(crate) fn save_scene(world: &mut World) {
    let Some(path) = ask_for_path(Dialog::Save) else {
        return;
    };

    let Some(text) = serialize(world) else {
        return;
    };
    if let Err(err) = std::fs::write(&path, text) {
        error!("could not write {}: {err}", path.display());
        return;
    }
    world.insert_resource(ProjectPath(Some(path)));
}

/// Prompts for a path and replaces whatever is loaded with what it
/// holds.
pub(crate) fn load_scene(world: &mut World) {
    let Some(path) = ask_for_path(Dialog::Open) else {
        return;
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            error!("could not read {}: {err}", path.display());
            return;
        }
    };

    let Some(project) = deserialize(world, &text, &path) else {
        return;
    };

    clear(world);

    let registry = world.resource::<AppTypeRegistry>().clone();
    if let Err(err) = project.world.write_to_world_with(
        world,
        &mut default(),
        &registry.read(),
    ) {
        error!("could not spawn {}: {err}", path.display());
        return;
    }

    // The recompile runs on `EditorScene` changing, so inserting it is
    // the whole of loading the animation.
    world.insert_resource(EditorScene::new(MotionGfxScene(
        project.scene,
    )));
    world.insert_resource(ProjectBookmarks(project.bookmarks));
    world.insert_resource(ProjectPath(Some(path)));
}

/// Everything a project file holds, in hand.
struct Project {
    world: DynamicWorld,
    scene: Scene<Backend>,
    bookmarks: Vec<PathBuf>,
}

fn serialize(world: &mut World) -> Option<String> {
    // The root comes too, or the `ChildOf` on everything below it
    // would name an entity the file never held.
    let subjects: Vec<Entity> = world
        .query_filtered::<Entity, Or<(With<EntityUid>, With<SceneRoot>)>>()
        .iter(world)
        .collect();

    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();

    let dynamic = DynamicWorldBuilder::from_world(world, &registry)
        .with_component_filter(subject_components())
        .extract_entities(subjects.into_iter())
        .build();

    let scene = world.resource::<EditorScene>();
    let bookmarks = world.resource::<ProjectBookmarks>();
    let document = Document {
        world: &dynamic,
        scene: &scene.scene().0,
        bookmarks: &bookmarks.0,
        registry: &registry,
    };

    match ron::ser::to_string_pretty(&document, pretty()) {
        Ok(text) => Some(text),
        Err(err) => {
            error!("could not serialize the project: {err}");
            None
        }
    }
}

fn deserialize(
    world: &mut World,
    text: &str,
    path: &Path,
) -> Option<Project> {
    let registry = world.resource::<AppTypeRegistry>().clone();
    let mut assets = world.resource::<AssetServer>().clone();

    let mut ron = match ron::de::Deserializer::from_str(text) {
        Ok(ron) => ron,
        Err(err) => {
            error!("{} is not valid RON: {err}", path.display());
            return None;
        }
    };

    let seed = ProjectSeed {
        registry: &registry.read(),
        assets: &mut assets,
    };
    match seed.deserialize(&mut ron) {
        Ok(project) => Some(project),
        Err(err) => {
            error!("could not read {}: {err}", path.display());
            None
        }
    }
}

/// Drops the loaded project, so nothing of it outlives the load.
///
/// Despawning [`SceneRoot`] is the whole of it, since every subject
/// hangs under it and bevy takes a despawned entity's descendants with
/// it. The selections go too: both name something in the scene being
/// replaced, and neither means anything in the one arriving.
fn clear(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<SceneRoot>>()
        .iter(world)
        .collect::<Vec<_>>();

    if roots.len() > 1 {
        warn!("There is more that one root in the world");
    }

    for root in roots {
        if let Ok(entity) = world.get_entity_mut(root) {
            entity.despawn();
        }
    }

    world.insert_resource(SelectedEntity(None));
    world.insert_resource(SelectedAction(None));
    world.insert_resource(ProjectBookmarks::default());
}

/// What a subject is saved as. An allowlist, not everything it
/// happens to carry: the rest is the running editor's business, and
/// a file that hoards it would not load into a different one.
fn subject_components() -> WorldFilter {
    WorldFilter::deny_all()
        .allow::<SceneRoot>()
        .allow::<EntityUid>()
        .allow::<Name>()
        .allow::<Transform>()
        .allow::<Visibility>()
        .allow::<Children>()
        .allow::<ChildOf>()
        .allow::<Camera3d>()
        .allow::<CascadeShadowConfig>()
        .allow::<DirectionalLight>()
        .allow::<PointLight>()
        .allow::<RectLight>()
        .allow::<SpotLight>()
        .allow::<Mesh3d>()
        .allow::<MeshMaterial3d<StandardMaterial>>()
        .allow::<Camera2d>()
}

enum Dialog {
    Open,
    Save,
}

/// Where a project file is picked, starting at the editor's own scene
/// folder. `None` when the dialog was dismissed.
fn ask_for_path(dialog: Dialog) -> Option<PathBuf> {
    let scenes = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../assets/scenes");
    let _ = std::fs::create_dir_all(&scenes);

    let file = rfd::FileDialog::new()
        .add_filter("MotionGfx project", &[EXTENSION])
        .set_directory(&scenes);

    match dialog {
        Dialog::Open => file.pick_file(),
        // A dialog that types the extension for you still lets it be
        // left off, and the loader finds a file by it.
        Dialog::Save => file
            .save_file()
            .map(|path| path.with_extension(EXTENSION)),
    }
}

fn pretty() -> ron::ser::PrettyConfig {
    ron::ser::PrettyConfig::default()
        .indentor("  ".to_string())
        .new_line("\n".to_string())
}

/// The project, on its way out.
///
/// Hand-written because a [`DynamicWorld`] needs the type registry to
/// serialize at all, which no derive can hand it.
struct Document<'a> {
    world: &'a DynamicWorld,
    scene: &'a Scene<Backend>,
    bookmarks: &'a [PathBuf],
    registry: &'a TypeRegistry,
}

impl Serialize for Document<'_> {
    fn serialize<S: Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut project = serializer.serialize_struct(PROJECT, 3)?;
        project.serialize_field(
            WORLD,
            &DynamicWorldSerializer::new(self.world, self.registry),
        )?;
        project.serialize_field(SCENE, self.scene)?;
        project.serialize_field(BOOKMARKS, self.bookmarks)?;
        project.end()
    }
}

/// The project, on its way in. Carries what the world half needs: the
/// registry to read components through, and somewhere for the asset
/// paths in them to be loaded from.
struct ProjectSeed<'a> {
    registry: &'a TypeRegistry,
    assets: &'a mut dyn LoadFromPath,
}

impl<'de> DeserializeSeed<'de> for ProjectSeed<'_> {
    type Value = Project;

    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_struct(
            PROJECT,
            &[WORLD, SCENE, BOOKMARKS],
            self,
        )
    }
}

impl<'de> Visitor<'de> for ProjectSeed<'_> {
    type Value = Project;

    fn expecting(
        &self,
        formatter: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        formatter.write_str("a motiongfx project")
    }

    fn visit_map<A: MapAccess<'de>>(
        self,
        mut map: A,
    ) -> Result<Self::Value, A::Error> {
        let mut world = None;
        let mut scene = None;
        let mut bookmarks = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                WORLD => {
                    world = Some(map.next_value_seed(
                        WorldDeserializer {
                            type_registry: self.registry,
                            load_from_path: self.assets,
                        },
                    )?);
                }
                SCENE => scene = Some(map.next_value()?),
                BOOKMARKS => bookmarks = Some(map.next_value()?),
                _ => {
                    map.next_value::<serde::de::IgnoredAny>()?;
                }
            }
        }

        Ok(Project {
            world: world.ok_or_else(|| {
                serde::de::Error::missing_field(WORLD)
            })?,
            scene: scene.ok_or_else(|| {
                serde::de::Error::missing_field(SCENE)
            })?,
            // Absent in a project saved before bookmarks existed.
            bookmarks: bookmarks.unwrap_or_default(),
        })
    }
}
