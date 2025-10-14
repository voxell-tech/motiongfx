use bevy_asset::prelude::*;
use bevy_ecs::{prelude::*, system::EntityCommands};
use bevy_hierarchy::prelude::*;
use bevy_math::prelude::*;
use bevy_render::prelude::*;
use bevy_transform::prelude::*;
use bevy_vello_renderer::{
    prelude::*,
    vello::{kurbo, peniko},
    vello_svg::usvg::{self, NodeExt},
};

use crate::{
    fill_style::FillStyle, stroke_style::StrokeStyle, vello_vector::bezpath::VelloBezPath,
};

/// Vello Bézier path group spawned from a Svg tree.
pub struct SvgTreeBundle {
    /// Parent entity of all the [`SvgPathBundle`]s.
    pub root_entity: Entity,
    /// Size of the Svg Tree.
    pub size: Vec2,
    /// All the [`SvgPathBundle`]s.
    pub paths: Vec<SvgPathBundle>,
}

impl SvgTreeBundle {
    pub fn new(root_entity: Entity, size: Vec2) -> Self {
        Self {
            root_entity,
            size,
            paths: Vec::new(),
        }
    }
}

/// Vello Bézier path spawned from a Svg path.
pub struct SvgPathBundle {
    /// Entity of the Svg path.
    pub entity: Entity,
    /// Transform of the Svg path.
    pub transform: Transform,
    /// Bézier path.
    pub path: kurbo::BezPath,
    /// [`FillStyle`] of the Svg.
    pub fill: Option<FillStyle>,
    /// [`StrokeStyle`] of the Svg.
    pub stroke: Option<StrokeStyle>,
}

impl SvgPathBundle {
    pub fn new(entity: Entity, transform: Transform) -> Self {
        Self {
            entity,
            transform,
            path: kurbo::BezPath::default(),
            fill: None,
            stroke: None,
        }
    }
}

/// Flattens the Svg hierarchy into a [`SvgTreeBundle`] while spawning the associated entities with corresponding components attached to them.
pub fn spawn_tree_flatten(
    commands: &mut Commands,
    scenes: &mut ResMut<Assets<VelloScene>>,
    svg: &usvg::Tree,
) -> SvgTreeBundle {
    let root_entity: Entity = commands
        .spawn((TransformBundle::default(), VisibilityBundle::default()))
        .id();

    let mut svg_tree_bundle =
        SvgTreeBundle::new(root_entity, Vec2::new(svg.size.width(), svg.size.height()));

    for node in svg.root.descendants() {
        // Only create entity for paths
        match &*node.borrow() {
            usvg::NodeKind::Group(_) => {}
            usvg::NodeKind::Path(path) => {
                let transform: Transform = svg_to_bevy_transform(node.abs_transform());
                let mut entity_commands: EntityCommands = commands.spawn((
                    TransformBundle::from_transform(transform),
                    VisibilityBundle::default(),
                ));

                let mut svg_path_bundle: SvgPathBundle =
                    SvgPathBundle::new(entity_commands.id(), transform);

                populate_with_path(&mut entity_commands, &mut svg_path_bundle, scenes, path);

                svg_tree_bundle.paths.push(svg_path_bundle);
            }
            usvg::NodeKind::Image(_) => {}
            usvg::NodeKind::Text(_) => {}
        }
    }

    let child_entities: &Vec<Entity> = &svg_tree_bundle
        .paths
        .iter()
        .map(|svg_path| svg_path.entity)
        .collect();

    commands.entity(root_entity).push_children(child_entities);

    svg_tree_bundle
}

fn populate_with_path(
    entity_commands: &mut EntityCommands,
    svg_path_bundle: &mut SvgPathBundle,
    scenes: &mut ResMut<Assets<VelloScene>>,
    path: &usvg::Path,
) {
    let mut local_path = kurbo::BezPath::new();
    // The semantics of SVG paths don't line up with `BezPath`; we must manually track initial points
    let mut just_closed: bool = false;
    let mut most_recent_initial = (0.0, 0.0);

    for elt in path.data.segments() {
        match elt {
            usvg::tiny_skia_path::PathSegment::MoveTo(p) => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                most_recent_initial = (p.x.into(), p.y.into());
                local_path.move_to(most_recent_initial)
            }
            usvg::tiny_skia_path::PathSegment::LineTo(p) => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                local_path.line_to(kurbo::Point::new(p.x as f64, p.y as f64))
            }
            usvg::tiny_skia_path::PathSegment::QuadTo(p1, p2) => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                local_path.quad_to(
                    kurbo::Point::new(p1.x as f64, p1.y as f64),
                    kurbo::Point::new(p2.x as f64, p2.y as f64),
                )
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                local_path.curve_to(
                    kurbo::Point::new(p1.x as f64, p1.y as f64),
                    kurbo::Point::new(p2.x as f64, p2.y as f64),
                    kurbo::Point::new(p3.x as f64, p3.y as f64),
                )
            }
            usvg::tiny_skia_path::PathSegment::Close => {
                just_closed = true;
                local_path.close_path()
            }
        }
    }

    entity_commands.insert(VelloBezPath::new(local_path.clone()));
    svg_path_bundle.path = local_path;

    // FIXME: let path.paint_order determine the fill/stroke order.

    if let Some(fill) = &path.fill {
        if let Some((brush, transform)) = paint_to_brush(&fill.paint, fill.opacity) {
            let fill_rule: peniko::Fill = match fill.rule {
                usvg::FillRule::NonZero => peniko::Fill::NonZero,
                usvg::FillRule::EvenOdd => peniko::Fill::EvenOdd,
            };
            let fill_style: FillStyle = FillStyle::new(fill_rule, brush, transform);

            entity_commands.insert(fill_style.clone());
            svg_path_bundle.fill = Some(fill_style);
        } else {
            // on_err(sb, &elt)?;
        }
    }

    if let Some(stroke) = &path.stroke {
        if let Some((brush, transform)) = paint_to_brush(&stroke.paint, stroke.opacity) {
            let mut conv_stroke: kurbo::Stroke = kurbo::Stroke::new(stroke.width.get() as f64)
                .with_caps(match stroke.linecap {
                    usvg::LineCap::Butt => kurbo::Cap::Butt,
                    usvg::LineCap::Round => kurbo::Cap::Round,
                    usvg::LineCap::Square => kurbo::Cap::Square,
                })
                .with_join(match stroke.linejoin {
                    usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => kurbo::Join::Miter,
                    usvg::LineJoin::Round => kurbo::Join::Round,
                    usvg::LineJoin::Bevel => kurbo::Join::Bevel,
                })
                .with_miter_limit(stroke.miterlimit.get() as f64);
            if let Some(dash_array) = stroke.dasharray.as_ref() {
                conv_stroke = conv_stroke.with_dashes(
                    stroke.dashoffset as f64,
                    dash_array.iter().map(|x| *x as f64),
                );
            }

            let stroke_style: StrokeStyle = StrokeStyle::new(conv_stroke, brush, transform);

            entity_commands.insert(stroke_style.clone());
            svg_path_bundle.stroke = Some(stroke_style);
        } else {
            // on_err(sb, &elt)?;
        }
    }

    entity_commands.insert(scenes.add(VelloScene::default()));
}

fn paint_to_brush(
    paint: &usvg::Paint,
    opacity: usvg::Opacity,
) -> Option<(peniko::Brush, kurbo::Affine)> {
    match paint {
        usvg::Paint::Color(color) => Some((
            peniko::Brush::Solid(peniko::Color::rgba8(
                color.red,
                color.green,
                color.blue,
                opacity.to_u8(),
            )),
            kurbo::Affine::IDENTITY,
        )),
        usvg::Paint::LinearGradient(gr) => {
            let stops: Vec<peniko::ColorStop> = gr
                .stops
                .iter()
                .map(|stop| {
                    let mut cstop = peniko::ColorStop::default();
                    cstop.color.r = stop.color.red;
                    cstop.color.g = stop.color.green;
                    cstop.color.b = stop.color.blue;
                    cstop.color.a = (stop.opacity * opacity).to_u8();
                    cstop.offset = stop.offset.get();
                    cstop
                })
                .collect();
            let start = kurbo::Point::new(gr.x1 as f64, gr.y1 as f64);
            let end = kurbo::Point::new(gr.x2 as f64, gr.y2 as f64);
            let arr = [
                gr.transform.sx,
                gr.transform.ky,
                gr.transform.kx,
                gr.transform.sy,
                gr.transform.tx,
                gr.transform.ty,
            ]
            .map(f64::from);
            let transform = kurbo::Affine::new(arr);
            let gradient = peniko::Gradient::new_linear(start, end).with_stops(stops.as_slice());
            Some((peniko::Brush::Gradient(gradient), transform))
        }
        usvg::Paint::RadialGradient(gr) => {
            let stops: Vec<peniko::ColorStop> = gr
                .stops
                .iter()
                .map(|stop| {
                    let mut cstop = peniko::ColorStop::default();
                    cstop.color.r = stop.color.red;
                    cstop.color.g = stop.color.green;
                    cstop.color.b = stop.color.blue;
                    cstop.color.a = (stop.opacity * opacity).to_u8();
                    cstop.offset = stop.offset.get();
                    cstop
                })
                .collect();

            let start_center = kurbo::Point::new(gr.cx as f64, gr.cy as f64);
            let end_center = kurbo::Point::new(gr.fx as f64, gr.fy as f64);
            let start_radius = 0_f32;
            let end_radius = gr.r.get();
            let arr = [
                gr.transform.sx,
                gr.transform.ky,
                gr.transform.kx,
                gr.transform.sy,
                gr.transform.tx,
                gr.transform.ty,
            ]
            .map(f64::from);
            let transform = kurbo::Affine::new(arr);
            let gradient = peniko::Gradient::new_two_point_radial(
                start_center,
                start_radius,
                end_center,
                end_radius,
            )
            .with_stops(stops.as_slice());
            Some((peniko::Brush::Gradient(gradient), transform))
        }
        usvg::Paint::Pattern(_) => None,
    }
}

fn svg_to_bevy_transform(transform: usvg::Transform) -> Transform {
    let usvg::Transform {
        sx,
        kx,
        ky,
        sy,
        tx,
        ty,
    } = transform;

    // https://stackoverflow.com/questions/39440369/how-to-convert-a-3x2-matrix-into-4x4-matrix
    let transform: [f32; 16] = [
        sx, kx, 0.0, 0.0, // row 1
        ky, sy, 0.0, 0.0, // row 2
        0.0, 0.0, 1.0, 0.0, // row 3
        tx, -ty, 0.0, 1.0, // row 4
    ];

    Transform::from_matrix(Mat4::from_cols_array(&transform))
}
