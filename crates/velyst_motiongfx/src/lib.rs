#![doc = include_str!("../README.md")]

use std::ops::Range;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::prelude::*;
use peniko_motiongfx::prelude::*;
use velyst::imaging::kurbo::{Shape, Vec2};
use velyst::kanva::Kanva;
use velyst::prelude::{VelystKanva, VelystSet};

pub use velyst;

pub mod prelude {
    pub use crate::{
        KanvaAnim, KanvaGroup, KanvaGroupKind, KanvaPhase,
    };

    pub use velyst::prelude::*;
}

pub struct VelystMotionGfxPlugin;

impl Plugin for VelystMotionGfxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (clear_kanva_mods, animate_kanva_anim)
                .chain()
                .in_set(VelystSet::PostLayout),
        );
    }
}

/// Identifies which [`VelystKanva`] entity and which paths within it
/// to target for animation.
///
/// Add alongside [`KanvaAnim`] to drive it.
#[derive(Component, Default)]
pub struct KanvaGroup {
    /// Target entity with [`VelystKanva`]. Uses self if `None`.
    pub target: Option<Entity>,
    pub kind: KanvaGroupKind,
}

impl KanvaGroup {
    pub fn all() -> Self {
        Self {
            kind: KanvaGroupKind::All,
            ..Default::default()
        }
    }

    pub fn inner(name: &'static str) -> Self {
        Self {
            kind: KanvaGroupKind::Inner(name),
            ..Default::default()
        }
    }

    pub fn wrap(start: &'static str, end: &'static str) -> Self {
        Self {
            kind: KanvaGroupKind::Wrap(start, end),
            ..Default::default()
        }
    }

    pub fn with_target(mut self, target: Entity) -> Self {
        self.target = Some(target);
        self
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

/// A single animation phase applied to each path in a [`KanvaGroup`].
///
/// `t_start` and `t_end` define where this phase sits within the path's
/// `[0, 1]` local-t window. The phase function receives a `[0, 1]` t
/// normalized to that sub-window.
pub struct KanvaPhase {
    /// Per-path animation function called with the normalized phase t.
    pub func: fn(&mut Kanva, path_idx: usize, t: f32),
    /// Start of this phase in the path's local-t space `[0, 1]`.
    pub t_start: f32,
    /// End of this phase in the path's local-t space `[0, 1]`.
    pub t_end: f32,
}

impl KanvaPhase {
    pub fn new(
        func: fn(&mut Kanva, usize, f32),
        t_start: f32,
        t_end: f32,
    ) -> Self {
        Self {
            func,
            t_start,
            t_end,
        }
    }
}

/// Drives staggered, multi-phase path animation on a [`KanvaGroup`].
#[derive(Component)]
pub struct KanvaAnim {
    /// Overall animation progress `[0, 1]`.
    pub t: f32,
    /// Fraction of `t` each path occupies (controls stagger density).
    pub path_window: f32,
    /// Ordered list of phases to apply per path.
    pub phases: Vec<KanvaPhase>,
}

impl KanvaAnim {
    /// Progressively reveals each path's stroke geometry.
    pub fn trace(path_window: f32) -> Self {
        Self {
            t: 0.0,
            path_window,
            phases: vec![KanvaPhase::new(trace_phase, 0.0, 1.0)],
        }
    }

    /// Fades each path in by its overall alpha.
    pub fn fade(path_window: f32) -> Self {
        Self {
            t: 0.0,
            path_window,
            phases: vec![KanvaPhase::new(alpha_phase, 0.0, 1.0)],
        }
    }

    /// Traces the stroke, then fades in the fill.
    ///
    /// `trace_ratio` controls how much of local-t is spent on tracing
    /// vs fill fade-in (e.g. `0.6` → 60 % trace, 40 % fill fade).
    pub fn trace_fade(path_window: f32, trace_ratio: f32) -> Self {
        Self {
            t: 0.0,
            path_window,
            phases: vec![
                KanvaPhase::new(trace_phase, 0.0, trace_ratio),
                KanvaPhase::new(fill_fade_phase, trace_ratio, 1.0),
            ],
        }
    }

    /// Scales each path up from half-size while fading its overall alpha in.
    pub fn scale_fade(path_window: f32) -> Self {
        Self {
            t: 0.0,
            path_window,
            phases: vec![
                KanvaPhase::new(scale_phase, 0.0, 1.0),
                KanvaPhase::new(alpha_phase, 0.0, 1.0),
            ],
        }
    }

    /// Slides each path up while fading its overall alpha in.
    pub fn fade_up(path_window: f32) -> Self {
        Self {
            t: 0.0,
            path_window,
            phases: vec![
                KanvaPhase::new(up_phase, 0.0, 1.0),
                KanvaPhase::new(alpha_phase, 0.0, 1.0),
            ],
        }
    }
}

/// Progressively reveals the path geometry (stroke-based trace animation).
pub fn trace_phase(kanva: &mut Kanva, path_idx: usize, t: f32) {
    let Some(orig) = kanva.get_path(path_idx).map(|p| p.path.clone())
    else {
        return;
    };
    kanva.mod_path(path_idx).shape(orig.trace(t));
}

/// Fades the path's fill color from transparent to fully opaque.
pub fn fill_fade_phase(kanva: &mut Kanva, path_idx: usize, t: f32) {
    let Some(path) = kanva.get_path(path_idx) else {
        return;
    };
    let fill = path.fill.and_then(|fi| kanva.get_fill(fi)).cloned();
    let faded = fill.map(|mut f| {
        f.brush = f.brush.with_alpha(t);
        f
    });
    kanva.mod_path(path_idx).fill(faded);
}

/// Fades the entire path (fill + stroke) from transparent to fully opaque.
pub fn alpha_phase(kanva: &mut Kanva, path_idx: usize, t: f32) {
    kanva.mod_path(path_idx).alpha(t);
}

/// Slides each path up from one bounding-box height below its final position.
pub fn up_phase(kanva: &mut Kanva, path_idx: usize, t: f32) {
    let Some(path) = kanva.get_path(path_idx) else {
        return;
    };

    let transform = path.transform;
    let t = ease::cubic::ease_in(1.0 - t);
    let offset = 50.0 * t as f64;
    kanva
        .mod_path(path_idx)
        .transform(transform.then_translate(Vec2::new(0.0, offset)));
}

pub fn scale_phase(kanva: &mut Kanva, path_idx: usize, t: f32) {
    let Some(path) = kanva.get_path(path_idx) else {
        return;
    };

    let transform = path.transform;
    let t = ease::cubic::ease_in(t);
    // Scale from the center.
    let scale = Interpolation::interp(&0.5_f64, &1.0_f64, t);
    let center = path.path.bounding_box().center();
    kanva.mod_path(path_idx).transform(
        transform
            .pre_translate(center.to_vec2())
            .pre_scale(scale)
            .pre_translate(-center.to_vec2()),
    );
}

fn resolve_range(
    kind: &KanvaGroupKind,
    kanva: &VelystKanva,
) -> Option<Range<usize>> {
    match kind {
        KanvaGroupKind::All => {
            let n = kanva.paths().len();
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

fn animate_kanva_anim(
    q: Query<(Entity, &KanvaAnim, &KanvaGroup)>,
    mut kanva_q: Query<&mut VelystKanva>,
) {
    for (entity, anim, group) in &q {
        let kanva_entity = group.target.unwrap_or(entity);
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
        let stagger = if n > 1 {
            (1.0 - anim.path_window) / (n - 1) as f32
        } else {
            0.0
        };

        for (i, path_idx) in range.enumerate() {
            // Clamp to [0, 1] so completed paths always hold their final
            // state rather than reverting to the raw Typst fill.
            let local_t = ((anim.t - i as f32 * stagger)
                / anim.path_window)
                .clamp(0.0, 1.0);

            for phase in &anim.phases {
                let span = phase.t_end - phase.t_start;
                let phase_t = if span <= 0.0 {
                    0.0
                } else {
                    ((local_t - phase.t_start) / span).clamp(0.0, 1.0)
                };
                (phase.func)(&mut kanva, path_idx, phase_t);
            }
        }
    }
}
