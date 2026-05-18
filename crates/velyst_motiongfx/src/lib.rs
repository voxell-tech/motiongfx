#![doc = include_str!("../README.md")]

use std::ops::Range;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::prelude::*;
use peniko_motiongfx::trace::Trace;
use velyst::prelude::{VelystKanva, VelystSet};

pub mod prelude {
    pub use crate::{
        KanvaGroup, KanvaGroupKind, TraceFadeKanva, TraceKanva,
        VelystMotionGfxPlugin,
    };
}

pub struct VelystMotionGfxPlugin;

impl Plugin for VelystMotionGfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (clear_kanva_mods, (animate_trace, animate_trace_fade))
                .chain()
                .in_set(VelystSet::PostLayout),
        );
    }
}

/// Identifies which [`VelystKanva`] entity and which paths within it
/// to target for animation.
///
/// Add alongside [`TraceKanva`] or [`TraceFadeKanva`] to drive them.
#[derive(Component, Default)]
pub struct KanvaGroup {
    /// Target entity with [`VelystKanva`]. Uses self if `None`.
    pub kanva: Option<Entity>,
    pub kind: KanvaGroupKind,
}

impl KanvaGroup {
    pub fn all(kanva: Entity) -> Self {
        Self { kanva: Some(kanva), kind: KanvaGroupKind::All }
    }

    pub fn inner(kanva: Entity, name: &'static str) -> Self {
        Self {
            kanva: Some(kanva),
            kind: KanvaGroupKind::Inner(name),
        }
    }

    pub fn wrap(
        kanva: Entity,
        start: &'static str,
        end: &'static str,
    ) -> Self {
        Self {
            kanva: Some(kanva),
            kind: KanvaGroupKind::Wrap(start, end),
        }
    }
}

#[derive(Default)]
pub enum KanvaGroupKind {
    /// All paths in the kanva.
    #[default]
    All,
    /// Paths inside a single labeled group.
    Inner(&'static str),
    /// Paths between two labeled group markers.
    Wrap(&'static str, &'static str),
}

#[derive(Component, Default)]
pub struct TraceKanva {
    pub t: f32,
    pub path_window: f32,
}

impl TraceKanva {
    pub fn new(path_window: f32) -> Self {
        Self {
            t: 0.0,
            path_window,
        }
    }
}

#[derive(Component)]
pub struct TraceFadeKanva {
    pub t: f32,
    pub path_window: f32,
    pub trace_ratio: f32,
}

impl Default for TraceFadeKanva {
    fn default() -> Self {
        Self {
            t: 0.0,
            path_window: 0.5,
            trace_ratio: 0.6,
        }
    }
}

fn resolve_range(
    kind: &KanvaGroupKind,
    kanva: &VelystKanva,
) -> Option<Range<usize>> {
    match kind {
        KanvaGroupKind::All => {
            let mut n = 0usize;
            while kanva.get_path(n).is_some() {
                n += 1;
            }
            (n > 0).then_some(0..n)
        }
        KanvaGroupKind::Inner(name) => {
            let idx = kanva.query_group(name)?;
            kanva.get_group_path_range(idx)
        }
        KanvaGroupKind::Wrap(start_name, end_name) => {
            let start_idx = kanva.query_group(start_name)?;
            let end_idx = kanva.query_group(end_name)?;
            kanva.get_paths_between_groups(start_idx, end_idx)
        }
    }
}

fn clear_kanva_mods(mut kanva_q: Query<&mut VelystKanva>) {
    for mut kanva in &mut kanva_q {
        kanva.clear_mods();
    }
}

fn animate_trace(
    q: Query<(Entity, &TraceKanva, &KanvaGroup)>,
    mut kanva_q: Query<&mut VelystKanva>,
) {
    for (entity, trace, group) in &q {
        let kanva_entity = group.kanva.unwrap_or(entity);
        let Ok(mut kanva) = kanva_q.get_mut(kanva_entity) else {
            continue;
        };
        if kanva.is_empty() {
            continue;
        }
        let Some(range) = resolve_range(&group.kind, &kanva) else {
            continue;
        };

        let n = range.len();
        let t = trace.t;
        let path_window = trace.path_window;
        let stagger = if n > 1 {
            (1.0 - path_window) / (n - 1) as f32
        } else {
            0.0
        };

        for (i, path_idx) in range.enumerate() {
            let local_t = (t - i as f32 * stagger) / path_window;
            if local_t >= 1.0 {
                continue;
            }
            let local_t = local_t.max(0.0);
            let Some(orig) =
                kanva.get_path(path_idx).map(|p| p.path.clone())
            else {
                continue;
            };
            kanva.mod_path(path_idx).shape(orig.trace(local_t));
        }
    }
}

fn animate_trace_fade(
    q: Query<(Entity, &TraceFadeKanva, &KanvaGroup)>,
    mut kanva_q: Query<&mut VelystKanva>,
) {
    for (entity, trace_fade, group) in &q {
        let kanva_entity = group.kanva.unwrap_or(entity);
        let Ok(mut kanva) = kanva_q.get_mut(kanva_entity) else {
            continue;
        };
        if kanva.is_empty() {
            continue;
        }
        let Some(range) = resolve_range(&group.kind, &kanva) else {
            continue;
        };

        let n = range.len();
        let t = trace_fade.t;
        let path_window = trace_fade.path_window;
        let trace_ratio = trace_fade.trace_ratio;
        let stagger = if n > 1 {
            (1.0 - path_window) / (n - 1) as f32
        } else {
            0.0
        };

        for (i, path_idx) in range.enumerate() {
            let local_t = (t - i as f32 * stagger) / path_window;
            if local_t >= 1.0 {
                continue;
            }
            let local_t = local_t.max(0.0);
            let Some(path) = kanva.get_path(path_idx) else {
                continue;
            };
            let orig = path.path.clone();
            let fill =
                path.fill.and_then(|fi| kanva.get_fill(fi)).cloned();

            let trace_t = (local_t / trace_ratio).min(1.0);
            let fade_t = ((local_t - trace_ratio)
                / (1.0 - trace_ratio))
                .clamp(0.0, 1.0);

            let faded_fill = fill.map(|mut f| {
                f.brush = f.brush.with_alpha(fade_t);
                f
            });
            kanva
                .mod_path(path_idx)
                .shape(orig.trace(trace_t))
                .fill(faded_fill);
        }
    }
}
