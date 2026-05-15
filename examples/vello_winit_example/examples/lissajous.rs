use core::f64;
use std::time::Instant;

use kurbo::{Affine, BezPath, Vec2};
use motiongfx::prelude::*;
use peniko::{Color, Fill};
use peniko_motiongfx::prelude::*;
use vello::peniko::color::palette::css;
use vello_winit_example::{VelloDemo, VelloWinitApp};
use winit::event_loop::EventLoop;

const N_X: usize = 12;
const N_Y: usize = 6;
const CELL_W: f64 = 110.0;
const CELL_H: f64 = 113.0;
const CURVE_R: f64 = 42.0;
const REF_R: f64 = 36.0;
const DELTA: f64 = f64::consts::PI / 4.0;
const DRAW_DUR: f32 = 1.5;
const HOLD_DUR: f32 = 0.8;
const STAGGER: f32 = 0.08; // per diagonal

fn curve_color(c: usize) -> Color {
    let t = (c - 1) as f64 / (N_X - 1) as f64;
    Color::from_rgba8(220, (t * 130.0) as u8, 0, 255)
}

fn lissajous_pt(
    a: f64,
    b: f64,
    t: f64,
    cx: f64,
    cy: f64,
) -> (f64, f64) {
    (
        cx + CURVE_R * (a * t + DELTA).sin(),
        cy + CURVE_R * (b * t).sin(),
    )
}

struct GridLine {
    line: kurbo::Line,
    width: f64,
    color: Color,
}

struct CurveState {
    tracer: PathTracer,
}

struct TableWorld {
    grid_lines: Vec<GridLine>,
    curves: Vec<CurveState>,
}

impl SubjectSource<usize, GridLine> for TableWorld {
    fn get_source(&self, id: usize) -> Option<&GridLine> {
        self.grid_lines.get(id)
    }

    fn apply_source<R>(
        &mut self,
        id: usize,
        f: impl FnOnce(&mut GridLine) -> R,
    ) -> Option<R> {
        self.grid_lines.get_mut(id).map(f)
    }
}

impl SubjectSource<usize, CurveState> for TableWorld {
    fn get_source(&self, id: usize) -> Option<&CurveState> {
        self.curves.get(id)
    }

    fn apply_source<R>(
        &mut self,
        id: usize,
        f: impl FnOnce(&mut CurveState) -> R,
    ) -> Option<R> {
        self.curves.get_mut(id).map(f)
    }
}

fn curve_id(c: usize, r: usize) -> usize {
    (r - 1) * N_X + (c - 1)
}

struct LissajousTableDemo {
    registry: Registry,
    world: TableWorld,
    start: Instant,
    timeline: Timeline<TableWorld>,
    grid_duration: f32,
    curve_duration: f32,
    window_size: kurbo::Size,
}

impl LissajousTableDemo {
    pub fn new() -> Self {
        let table_w = (N_X + 1) as f64 * CELL_W;
        let table_h = (N_Y + 1) as f64 * CELL_H;

        let mut registry = Registry::new();
        let mut world = TableWorld {
            grid_lines: Vec::new(),
            curves: (0..N_X * N_Y)
                .map(|id| {
                    let c = id % N_X + 1;
                    let r = id / N_X + 1;
                    let a = c as f64;
                    let b = r as f64;
                    let cx = c as f64 * CELL_W + CELL_W / 2.0;
                    let cy = r as f64 * CELL_H + CELL_H / 2.0;
                    let samples = a.max(b) as usize * 80 + 2;
                    let mut path = BezPath::new();
                    let (px, py) = lissajous_pt(a, b, 0.0, cx, cy);
                    path.move_to((px, py));
                    for i in 1..=samples {
                        let t = i as f64 / samples as f64
                            * f64::consts::TAU;
                        let (px, py) =
                            lissajous_pt(a, b, t, cx, cy);
                        path.line_to((px, py));
                    }
                    CurveState {
                        tracer: PathTracer {
                            path,
                            t_start: 0.0,
                            t_end: 0.0,
                        },
                    }
                })
                .collect(),
        };

        // Grid lines: p1 starts at p0 (zero length) so they draw in.
        let grid_col = Color::from_rgba8(105, 105, 140, 255);
        let mut vert_entries: Vec<(usize, kurbo::Point)> = Vec::new();
        let mut horiz_entries: Vec<(usize, kurbo::Point)> =
            Vec::new();
        for c in 0..=(N_X + 1) {
            let x = c as f64 * CELL_W;
            let p0 = kurbo::Point::new(x, 0.0);
            let id = world.grid_lines.len();
            world.grid_lines.push(GridLine {
                line: kurbo::Line::new(p0, p0),
                width: 0.5,
                color: grid_col,
            });
            vert_entries.push((id, kurbo::Point::new(x, table_h)));
        }
        for r in 0..=(N_Y + 1) {
            let y = r as f64 * CELL_H;
            let p0 = kurbo::Point::new(0.0, y);
            let id = world.grid_lines.len();
            world.grid_lines.push(GridLine {
                line: kurbo::Line::new(p0, p0),
                width: 0.5,
                color: grid_col,
            });
            horiz_entries.push((id, kurbo::Point::new(table_w, y)));
        }

        let mut b = registry.create_builder::<TableWorld>();

        // Track 0: one column + one row draw in simultaneously per step.
        let n_steps = vert_entries.len().max(horiz_entries.len());
        let mut pair_tracks = Vec::with_capacity(n_steps);
        for i in 0..n_steps {
            let v = vert_entries.get(i).map(|&(id, p1)| {
                b.act(id, path!(<GridLine>::line::p1), move |_| p1)
                    .with_ease(ease::cubic::ease_in_out)
                    .play(0.6)
            });
            let h = horiz_entries.get(i).map(|&(id, p1)| {
                b.act(id, path!(<GridLine>::line::p1), move |_| p1)
                    .with_ease(ease::cubic::ease_in_out)
                    .play(0.6)
            });
            pair_tracks.push(match (v, h) {
                (Some(v), Some(h)) => [v, h].ord_flow(0.025),
                (Some(v), None) => v,
                (None, Some(h)) => h,
                (None, None) => unreachable!(),
            });
        }
        let grid_track =
            pair_tracks.into_iter().ord_flow(0.05).compile();
        b.add_tracks(grid_track);

        // Track 1: curves draw in and out, looped by the caller.
        let max_diag = N_X + N_Y;
        let curve_track = (2..=max_diag)
            .map(|d| {
                (1..=N_X)
                    .filter_map(|c| {
                        let r = d.checked_sub(c)?;
                        if !(1..=N_Y).contains(&r) {
                            return None;
                        }
                        let id = curve_id(c, r);
                        let draw_in = b
                            .act(
                                id,
                                path!(<CurveState>::tracer::t_end),
                                |_| 1.0f32,
                            )
                            .with_ease(ease::cubic::ease_in_out)
                            .play(DRAW_DUR);
                        let draw_out = b
                            .act(
                                id,
                                path!(<CurveState>::tracer::t_start),
                                |_| 1.0f32,
                            )
                            .with_ease(ease::cubic::ease_in_out)
                            .play(DRAW_DUR);
                        Some(
                            [draw_in, draw_out]
                                .ord_flow(DRAW_DUR + HOLD_DUR),
                        )
                    })
                    .ord_all()
            })
            .ord_flow(STAGGER)
            .compile();
        b.add_tracks(curve_track);

        let mut timeline = b.compile();
        timeline.bake_actions(&registry, &world);
        let grid_duration = timeline.tracks()[0].duration();
        let curve_duration = timeline.tracks()[1].duration();

        Self {
            registry,
            world,
            start: Instant::now(),
            timeline,
            grid_duration,
            curve_duration,
            window_size: kurbo::Size::new(
                (N_X + 1) as f64 * CELL_W,
                (N_Y + 1) as f64 * CELL_H,
            ),
        }
    }
}

impl VelloDemo for LissajousTableDemo {
    fn window_title(&self) -> &'static str {
        "Lissajous Table"
    }

    fn initial_logical_size(&self) -> (f64, f64) {
        ((N_X + 1) as f64 * CELL_W, (N_Y + 1) as f64 * CELL_H)
    }

    fn size_changed(&mut self, size: kurbo::Size) {
        self.window_size = size;
    }

    fn rebuild_scene(
        &mut self,
        scene: &mut vello::Scene,
        scale_factor: f64,
    ) {
        let elapsed = self.start.elapsed().as_secs_f32();

        // Track 0 plays once; once done we lock to track 1 and loop it.
        if elapsed < self.grid_duration {
            self.timeline.set_target_track(0);
            self.timeline.set_target_time(elapsed);
        } else {
            self.timeline.set_target_track(1);
            self.timeline.set_target_time(
                (elapsed - self.grid_duration) % self.curve_duration,
            );
        }
        self.timeline.queue_actions();
        self.timeline
            .sample_queued_actions(&self.registry, &mut self.world);

        // Centre the table in the window.
        let table_w = (N_X + 1) as f64 * CELL_W;
        let table_h = (N_Y + 1) as f64 * CELL_H;
        let ox = (self.window_size.width - table_w).max(0.0) / 2.0;
        let oy = (self.window_size.height - table_h).max(0.0) / 2.0;
        let xf = Affine::scale(scale_factor)
            .pre_translate(Vec2::new(ox, oy));

        // Grid lines
        for line in &self.world.grid_lines {
            scene.stroke(
                &kurbo::Stroke::new(line.width),
                xf,
                line.color,
                None,
                &line.line,
            );
        }

        // Reference circles: dot tracks the curve tip of row 1 / col 1.
        let xhair_col = Color::from_rgba8(90, 90, 110, 255);
        let xhair_st = kurbo::Stroke::new(0.5);
        let ref_st = kurbo::Stroke::new(1.0);

        for c in 1..=N_X {
            let a = c as f64;
            let cx = c as f64 * CELL_W + CELL_W / 2.0;
            let cy = CELL_H / 2.0;
            scene.stroke(
                &ref_st,
                xf,
                css::GAINSBORO,
                None,
                &kurbo::Circle::new((cx, cy), REF_R),
            );
            scene.stroke(
                &xhair_st,
                xf,
                xhair_col,
                None,
                &kurbo::Line::new((cx, cy - REF_R), (cx, cy + REF_R)),
            );
            scene.stroke(
                &xhair_st,
                xf,
                xhair_col,
                None,
                &kurbo::Line::new((cx - REF_R, cy), (cx + REF_R, cy)),
            );
            // x-component of curve tip: sin(a * t_end + DELTA)
            let t_end =
                self.world.curves[curve_id(c, 1)].tracer.t_end as f64
                    * f64::consts::TAU;
            let angle = a * t_end + DELTA - f64::consts::FRAC_PI_2;
            scene.fill(
                Fill::NonZero,
                xf,
                css::WHITE,
                None,
                &kurbo::Circle::new(
                    (
                        cx + REF_R * angle.cos(),
                        cy + REF_R * angle.sin(),
                    ),
                    3.5,
                ),
            );
        }

        for r in 1..=N_Y {
            let b = r as f64;
            let cx = CELL_W / 2.0;
            let cy = r as f64 * CELL_H + CELL_H / 2.0;
            scene.stroke(
                &ref_st,
                xf,
                css::GAINSBORO,
                None,
                &kurbo::Circle::new((cx, cy), REF_R),
            );
            scene.stroke(
                &xhair_st,
                xf,
                xhair_col,
                None,
                &kurbo::Line::new((cx, cy - REF_R), (cx, cy + REF_R)),
            );
            scene.stroke(
                &xhair_st,
                xf,
                xhair_col,
                None,
                &kurbo::Line::new((cx - REF_R, cy), (cx + REF_R, cy)),
            );
            // y-component of curve tip: sin(b * t_end)
            let t_end =
                self.world.curves[curve_id(1, r)].tracer.t_end as f64
                    * f64::consts::TAU;
            let angle = b * t_end - f64::consts::FRAC_PI_2;
            scene.fill(
                Fill::NonZero,
                xf,
                css::WHITE,
                None,
                &kurbo::Circle::new(
                    (
                        cx + REF_R * angle.cos(),
                        cy + REF_R * angle.sin(),
                    ),
                    3.5,
                ),
            );
        }

        // Lissajous curves driven by timeline world state
        for r in 1..=N_Y {
            for c in 1..=N_X {
                let id = curve_id(c, r);
                let state = &self.world.curves[id];

                const MIN_ARC: f32 = 0.01;
                if state.tracer.t_end - state.tracer.t_start < MIN_ARC
                    || 1.0 - state.tracer.t_start < MIN_ARC
                {
                    continue;
                }

                let visible = state.tracer.trace();
                scene.stroke(
                    &kurbo::Stroke::new(2.0),
                    xf,
                    curve_color(c),
                    None,
                    &visible,
                );
            }
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = VelloWinitApp::new(LissajousTableDemo::new());
    event_loop.run_app(&mut app).unwrap();
}
