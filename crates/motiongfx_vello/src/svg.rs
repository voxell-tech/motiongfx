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

pub fn spawn_tree(
    commands: &mut Commands,
    fragments: &mut ResMut<Assets<VelloFragment>>,
    svg: &usvg::Tree,
) -> Entity {
    commands
        .spawn((
            TransformBundle::from_transform(svg_transform(svg.root.abs_transform())),
            VisibilityBundle::default(),
        ))
        .with_children(|parent| {
            if svg.root.has_children() {
                for child in svg.root.children() {
                    spawn_child_recursive(parent, fragments, child);
                }
            }
        })
        .id()
}

/// Flattens the Svg hierarchy into a [`SvgTreeBundle`] while spawning the associated entities with corresponding components attached to them.
pub fn spawn_tree_flatten(
    commands: &mut Commands,
    fragments: &mut ResMut<Assets<VelloFragment>>,
    svg: &usvg::Tree,
) -> SvgTreeBundle {
    let root_entity: Entity = commands
        .spawn((TransformBundle::default(), VisibilityBundle::default()))
        .id();

    let mut svg_tree_bundle = SvgTreeBundle::new(
        root_entity,
        Vec2::new(svg.size.width() as f32, svg.size.height() as f32),
    );

    for node in svg.root.descendants() {
        // Only create entity for paths
        match &*node.borrow() {
            usvg::NodeKind::Group(_) => {}
            usvg::NodeKind::Path(path) => {
                let transform: Transform = svg_transform(node.abs_transform());
                let mut entity_commands: EntityCommands = commands.spawn((
                    TransformBundle::from_transform(transform),
                    VisibilityBundle::default(),
                ));

                let mut svg_path_bundle: SvgPathBundle =
                    SvgPathBundle::new(entity_commands.id(), transform);

                populate_with_path(&mut entity_commands, &mut svg_path_bundle, fragments, path);

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

    commands.entity(root_entity).push_children(&child_entities);

    svg_tree_bundle
}

fn spawn_child_recursive(
    parent: &mut ChildBuilder,
    fragments: &mut ResMut<Assets<VelloFragment>>,
    node: usvg::Node,
) {
    let transform: Transform = svg_transform(node.transform());
    let mut entity_commands: EntityCommands = parent.spawn((
        TransformBundle::from_transform(transform),
        VisibilityBundle::default(),
    ));

    match &*node.borrow() {
        usvg::NodeKind::Group(_) => {}
        usvg::NodeKind::Path(path) => {
            let mut svg_path_bundle: SvgPathBundle =
                SvgPathBundle::new(entity_commands.id(), transform);

            populate_with_path(&mut entity_commands, &mut svg_path_bundle, fragments, path);
        }
        usvg::NodeKind::Image(_) => {}
        usvg::NodeKind::Text(_) => {}
    }

    if node.has_children() {
        for child_node in node.children() {
            entity_commands.with_children(|child_parent| {
                spawn_child_recursive(child_parent, fragments, child_node);
            });
        }
    }
}

fn populate_with_path(
    entity_commands: &mut EntityCommands,
    svg_path_bundle: &mut SvgPathBundle,
    fragments: &mut ResMut<Assets<VelloFragment>>,
    path: &usvg::Path,
) {
    let mut local_path = kurbo::BezPath::new();
    // The semantics of SVG paths don't line up with `BezPath`; we must manually track initial points
    let mut just_closed: bool = false;
    let mut most_recent_initial: kurbo::Point = kurbo::Point::default();

    for elt in path.data.segments() {
        match elt {
            usvg::PathSegment::MoveTo { x, y } => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                most_recent_initial = kurbo::Point::new(x, y);
                local_path.move_to(most_recent_initial)
            }
            usvg::PathSegment::LineTo { x, y } => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                local_path.line_to((x, y))
            }
            usvg::PathSegment::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                if std::mem::take(&mut just_closed) {
                    local_path.move_to(most_recent_initial);
                }
                local_path.curve_to((x1, y1), (x2, y2), (x, y))
            }
            usvg::PathSegment::ClosePath => {
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
            // FIXME: Set the fill rule
            let fill_style: FillStyle = FillStyle::new(peniko::Fill::NonZero, brush, transform);

            entity_commands.insert(fill_style.clone());
            svg_path_bundle.fill = Some(fill_style);
        } else {
            // on_err(sb, &elt)?;
        }
    }

    if let Some(stroke) = &path.stroke {
        if let Some((brush, transform)) = paint_to_brush(&stroke.paint, stroke.opacity) {
            // FIXME: handle stroke options such as linecap, linejoin, etc.
            let stroke_style: StrokeStyle = StrokeStyle::new(stroke.width.get(), brush, transform);

            entity_commands.insert(stroke_style.clone());
            svg_path_bundle.stroke = Some(stroke_style);
        } else {
            // on_err(sb, &elt)?;
        }
    }

    entity_commands.insert(fragments.add(VelloFragment::default()));
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
                    cstop.offset = stop.offset.get() as f32;
                    cstop
                })
                .collect();
            let start: kurbo::Point = (gr.x1, gr.y1).into();
            let end: kurbo::Point = (gr.x2, gr.y2).into();
            let transform = kurbo::Affine::new([
                gr.transform.a,
                gr.transform.b,
                gr.transform.c,
                gr.transform.d,
                gr.transform.e,
                gr.transform.f,
            ]);
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
                    cstop.offset = stop.offset.get() as f32;
                    cstop
                })
                .collect();

            let start_center: kurbo::Point = (gr.fx, gr.fy).into();
            let end_center: kurbo::Point = (gr.cx, gr.cy).into();
            let start_radius = 0_f32;
            let end_radius = gr.r.get() as f32;
            let transform = kurbo::Affine::new([
                gr.transform.a,
                gr.transform.b,
                gr.transform.c,
                gr.transform.d,
                gr.transform.e,
                gr.transform.f,
            ]);
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

fn svg_transform(transform: usvg::Transform) -> Transform {
    let usvg::Transform { a, b, c, d, e, f } = transform;

    // https://stackoverflow.com/questions/39440369/how-to-convert-a-3x2-matrix-into-4x4-matrix
    let transform: [f32; 16] = [
        a as f32, b as f32, 0.0, 0.0, // row 1
        c as f32, d as f32, 0.0, 0.0, // row 2
        0.0, 0.0, 1.0, 0.0, // row 3
        e as f32, -f as f32, 0.0, 1.0, // row 4
    ];

    Transform::from_matrix(Mat4::from_cols_array(&transform))
}
