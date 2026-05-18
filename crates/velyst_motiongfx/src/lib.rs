#![doc = include_str!("../README.md")]

use std::ops::Range;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::prelude::*;
use peniko_motiongfx::trace::Trace;
use velyst::kanva::Kanva;
use velyst::prelude::{VelystKanva, VelystSet};

pub mod prelude {
    pub use crate::{
        KanvaGroup, TraceKanva, TraceFadeKanva, VelystMotionGfxPlugin,
    };
}

pub struct VelystMotionGfxPlugin;

impl Plugin for VelystMotionGfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                clear_kanva_mods,
                (animate_trace, animate_trace_fade),
            )
                .chain()
                .in_set(VelystSet::PostLayout),
        );
    }
}

pub enum KanvaGroup {
    /// Explicit start and end group markers.
    Wrap(&'static str, &'static str),
    /// Single group name.
    Inner(&'static str),
    /// All paths in the kanva.
    All,
}

#[derive(Component)]
pub struct TraceKanva {
    pub t: f32,
    pub path_window: f32,
    pub kanva: Option<Entity>,
    pub group: KanvaGroup,
}

impl Default for TraceKanva {
    fn default() -> Self {
        Self {
            t: 0.0,
            path_window: 0.3,
            kanva: None,
            group: KanvaGroup::All,
        }
    }
}

#[derive(Component)]
pub struct TraceFadeKanva {
    pub t: f32,
    pub path_window: f32,
    pub trace_ratio: f32,
    pub kanva: Option<Entity>,
    pub group: KanvaGroup,
}

impl Default for TraceFadeKanva {
    fn default() -> Self {
        Self {
            t: 0.0,
            path_window: 0.5,
            trace_ratio: 0.6,
            kanva: None,
            group: KanvaGroup::All,
        }
    }
}

fn resolve_range(
    group: &KanvaGroup,
    kanva: &Kanva,
) -> Option<Range<usize>> {
    match group {
        KanvaGroup::Wrap(start_name, end_name) => {
            let start_idx = kanva.query_group(start_name)?;
            let end_idx = kanva.query_group(end_name)?;
            kanva.get_paths_between_groups(start_idx, end_idx)
        }
        KanvaGroup::Inner(name) => {
            let idx = kanva.query_group(name)?;
            kanva.get_group_path_range(idx)
        }
        KanvaGroup::All => {
            let mut n = 0usize;
            while kanva.get_path(n).is_some() {
                n += 1;
            }
            (n > 0).then_some(0..n)
        }
    }
}

fn clear_kanva_mods(mut kanva_q: Query<&mut VelystKanva>) {
    for mut kanva in &mut kanva_q {
        kanva.clear_mods();
    }
}

fn animate_trace(
    q: Query<(Entity, &TraceKanva)>,
    mut kanva_q: Query<&mut VelystKanva>,
) {
    for (entity, trace) in &q {
        let kanva_entity = trace.kanva.unwrap_or(entity);
        let Ok(mut kanva) = kanva_q.get_mut(kanva_entity) else {
            continue;
        };
        if kanva.is_empty() {
            continue;
        }
        let Some(range) = resolve_range(&trace.group, &kanva) else {
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
    q: Query<(Entity, &TraceFadeKanva)>,
    mut kanva_q: Query<&mut VelystKanva>,
) {
    for (entity, trace_fade) in &q {
        let kanva_entity = trace_fade.kanva.unwrap_or(entity);
        let Ok(mut kanva) = kanva_q.get_mut(kanva_entity) else {
            continue;
        };
        if kanva.is_empty() {
            continue;
        }
        let Some(range) =
            resolve_range(&trace_fade.group, &kanva)
        else {
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
