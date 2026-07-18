//! "+" button popup: lists registered windows; clicking one adds it
//! as a tab to that area's leaf (the reconciler rebuilds the UI).

use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

use super::drag::logical_rect;
use super::reconcile::NodeBinding;
use super::registry::WindowRegistry;
use super::tabs::DockTabAddButton;
use super::tree::DockTree;
use crate::ui::glass::{Glass, glass_button};
use crate::ui::theme::EditorTheme;

pub struct AddWindowPopupPlugin;

impl Plugin for AddWindowPopupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_add_clicks);
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

        commands
            .spawn((
                AddWindowPopupBackdrop,
                // Catch the outside-click to close, but let hover/clicks
                // pass through to the UI beneath instead of freezing it.
                Pickable {
                    should_block_lower: false,
                    is_hoverable: true,
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                GlobalZIndex(180),
            ))
            .observe(
                |mut click: On<Pointer<Click>>,
                 q_open: Query<
                    Entity,
                    Or<(
                        With<AddWindowPopup>,
                        With<AddWindowPopupBackdrop>,
                    )>,
                >,
                 mut commands: Commands| {
                    click.propagate(false);
                    for open in &q_open {
                        commands.entity(open).try_despawn();
                    }
                },
            );

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
            // Each row is a full-width glass button; the click handler
            // captures the window id + target area directly instead of
            // going through a component (which would need `Entity`'s
            // absent `Default` impl for the bsn template system).
            let window_id = desc.id.clone();
            let area = button.area_entity;
            commands.spawn_scene(bsn! {
                glass_button()
                on(move |mut click: On<Pointer<Click>>,
                         q_bindings: Query<&NodeBinding>,
                         q_open: Query<
                            Entity,
                            Or<(With<AddWindowPopup>, With<AddWindowPopupBackdrop>)>,
                         >,
                         mut tree: ResMut<DockTree>,
                         mut commands: Commands| {
                    click.propagate(false);
                    if let Ok(binding) = q_bindings.get(area) {
                        tree.add_tab(binding.0, window_id.clone());
                    }
                    for open in &q_open {
                        commands.entity(open).try_despawn();
                    }
                })
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                }
                ChildOf({popup})
                Children [(
                    Text({desc.name.clone()})
                    TextFont { font_size: FontSize::Px(12.0) }
                    TextColor({theme.text_primary})
                )]
            });
        }
    }
}
