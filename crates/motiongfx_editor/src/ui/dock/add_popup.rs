//! "+" button popup: lists registered windows; clicking one adds it
//! as a tab to that area's leaf (the reconciler rebuilds the UI).

use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy::window::SystemCursorIcon;

use super::drag::logical_rect;
use super::reconcile::NodeBinding;
use super::registry::WindowRegistry;
use super::tabs::DockTabAddButton;
use super::tree::DockTree;
use crate::ui::glass::Glass;
use crate::ui::theme::EditorTheme;

pub struct AddWindowPopupPlugin;

impl Plugin for AddWindowPopupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (handle_add_clicks, handle_item_clicks, hover_items),
        );
    }
}

const POPUP_WIDTH: f32 = 150.0;

/// Root of the open popup (despawned on any close path). Carries the
/// "+" button that opened it, so a second press on it toggles closed.
#[derive(Component)]
pub struct AddWindowPopup {
    owner: Entity,
}

/// Full-screen click-catcher behind the popup.
#[derive(Component)]
pub struct AddWindowPopupBackdrop;

/// One selectable window entry.
#[derive(Component)]
pub struct AddWindowPopupItem {
    window_id: String,
    /// The dock area whose leaf receives the new tab.
    area: Entity,
}

/// Open the popup under the pressed "+" button; pressing the same
/// button again closes it instead.
#[expect(clippy::type_complexity)]
fn handle_add_clicks(
    q_buttons: Query<
        (
            Entity,
            &DockTabAddButton,
            &Interaction,
            &ComputedNode,
            &UiGlobalTransform,
        ),
        Changed<Interaction>,
    >,
    q_popup: Query<&AddWindowPopup>,
    q_open: Query<
        Entity,
        Or<(With<AddWindowPopup>, With<AddWindowPopupBackdrop>)>,
    >,
    registry: Res<WindowRegistry>,
    tree: Res<DockTree>,
    theme: Res<EditorTheme>,
    mut commands: Commands,
) {
    for (button_entity, button, interaction, computed, transform) in
        &q_buttons
    {
        if *interaction != Interaction::Pressed {
            continue;
        }
        // `try_despawn`: the backdrop-close path may despawn these
        // same entities this frame (a "+" click also hits the
        // backdrop).
        for open in &q_open {
            commands.entity(open).try_despawn();
        }
        // Second press on the same button: toggle closed.
        if q_popup.iter().any(|p| p.owner == button_entity) {
            continue;
        }

        commands.spawn((
            AddWindowPopupBackdrop,
            Interaction::default(),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(180),
        ));

        // Right-align the popup to the button, just below it.
        let rect = logical_rect(computed, transform);
        let popup = commands
            .spawn((
                AddWindowPopup {
                    owner: button_entity,
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(rect.max.x - POPUP_WIDTH),
                    top: Val::Px(rect.max.y + 4.0),
                    width: Val::Px(POPUP_WIDTH),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(4.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                Glass::Popup,
                GlobalZIndex(181),
            ))
            .id();

        // Windows are single-instance: only closed ones are listed.
        for desc in registry
            .iter()
            .filter(|d| tree.find_leaf_with_window(&d.id).is_none())
        {
            commands.spawn((
                AddWindowPopupItem {
                    window_id: desc.id.clone(),
                    area: button.area_entity,
                },
                Interaction::default(),
                EntityCursor::System(SystemCursorIcon::Pointer),
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                ChildOf(popup),
                children![(
                    Text::new(desc.name.clone()),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme.text_muted),
                )],
            ));
        }
    }
}

/// Item picked: add the tab to the area's leaf. Backdrop press just
/// closes. Either way the popup is torn down.
fn handle_item_clicks(
    q_items: Query<(&AddWindowPopupItem, &Interaction), Changed<Interaction>>,
    q_backdrop: Query<&Interaction, (With<AddWindowPopupBackdrop>, Changed<Interaction>)>,
    q_open: Query<
        Entity,
        Or<(With<AddWindowPopup>, With<AddWindowPopupBackdrop>)>,
    >,
    q_bindings: Query<&NodeBinding>,
    mut tree: ResMut<DockTree>,
    mut commands: Commands,
) {
    let mut close = false;

    for (item, interaction) in &q_items {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Ok(binding) = q_bindings.get(item.area) {
            tree.add_tab(binding.0, item.window_id.clone());
        }
        close = true;
    }
    if q_backdrop.iter().any(|i| *i == Interaction::Pressed) {
        close = true;
    }

    if close {
        for open in &q_open {
            commands.entity(open).try_despawn();
        }
    }
}

/// Hover highlight for popup rows.
fn hover_items(
    theme: Res<EditorTheme>,
    mut q_items: Query<
        (&Interaction, &mut BackgroundColor),
        (With<AddWindowPopupItem>, Changed<Interaction>),
    >,
) {
    for (interaction, mut bg) in &mut q_items {
        bg.0 = match interaction {
            Interaction::None => Color::NONE,
            _ => theme.hover_fill,
        };
    }
}
