//! Scene hierarchy browser: an indented list of the scene's entities.
//!
//! Only *scene* entities are listed: anything with a [`Transform`]
//! that isn't part of the editor's own `bevy_ui` tree, so the panel
//! shows the composition's objects rather than the editor chrome.

use bevy::prelude::*;
use motiongfx_editor_ui::theme::EditorTheme;

use crate::scene::TrackViewportCamera;

/// Indent per hierarchy level.
const INDENT: f32 = 12.0;

/// Marks the scrollable panel the rows are built into.
#[derive(Component, Default, Clone)]
pub(crate) struct HierarchyPanel;

/// Marks a generated row, for teardown on rebuild.
#[derive(Component, Default, Clone)]
pub(crate) struct HierarchyRow;

/// The rows currently displayed, to detect when a rebuild is needed.
#[derive(Resource, Default)]
pub(crate) struct HierarchyState {
    shown: Vec<(Entity, usize)>,
}

/// Scene entities: transform-bearing and not editor UI.
type SceneEntity =
    (With<Transform>, Without<Node>, Without<TrackViewportCamera>);

/// Rebuild the list when the scene's entities, their nesting, or their
/// names change (or when the tab is re-materialized elsewhere).
pub(crate) fn build_hierarchy_view(
    mut commands: Commands,
    mut state: ResMut<HierarchyState>,
    theme: Res<EditorTheme>,
    q_panel: Query<Entity, With<HierarchyPanel>>,
    q_panel_added: Query<(), Added<HierarchyPanel>>,
    q_renamed: Query<(), (Changed<Name>, SceneEntity)>,
    q_scene: Query<(Entity, Option<&Children>), SceneEntity>,
    q_names: Query<&Name>,
    q_parents: Query<&ChildOf>,
    q_rows: Query<Entity, With<HierarchyRow>>,
) {
    let Ok(panel) = q_panel.single() else {
        return;
    };

    // Roots first (a scene entity whose parent isn't itself a scene
    // entity), then depth-first so children follow their parent.
    let mut rows = Vec::new();
    let mut roots = q_scene
        .iter()
        .map(|(entity, _)| entity)
        .filter(|&entity| {
            !q_parents
                .get(entity)
                .is_ok_and(|p| q_scene.contains(p.parent()))
        })
        .collect::<Vec<_>>();
    roots.sort_unstable();
    for root in roots {
        push_subtree(root, 0, &q_scene, &mut rows);
    }

    let dirty = rows != state.shown
        || !q_panel_added.is_empty()
        || !q_renamed.is_empty();
    if !dirty {
        return;
    }

    for row in &q_rows {
        commands.entity(row).despawn();
    }

    for &(entity, depth) in &rows {
        let name = q_names
            .get(entity)
            .map(|n| n.as_str().to_string())
            .unwrap_or_else(|_| format!("Entity {}", entity.index()));
        commands.spawn_scene(bsn! {
            HierarchyRow
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                padding: UiRect::left(Val::Px(depth as f32 * INDENT)),
            }
            ChildOf({panel})
            Children [(
                Text({name})
                TextFont { font_size: FontSize::Px(12.0) }
                TextColor({theme.text_primary})
            )]
        });
    }

    state.shown = rows;
}

/// Depth-first append `entity` and its scene descendants.
fn push_subtree(
    entity: Entity,
    depth: usize,
    q_scene: &Query<(Entity, Option<&Children>), SceneEntity>,
    out: &mut Vec<(Entity, usize)>,
) {
    let Ok((_, children)) = q_scene.get(entity) else {
        return;
    };
    out.push((entity, depth));
    for child in children.into_iter().flatten() {
        push_subtree(*child, depth + 1, q_scene, out);
    }
}
