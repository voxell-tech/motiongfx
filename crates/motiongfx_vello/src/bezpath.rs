use bevy_ecs::prelude::*;
// use bevy_math::prelude::*;
use bevy_reflect::{TypePath, TypeUuid};
use bevy_vello_renderer::{
    vello::kurbo::{BezPath, PathEl},
    vello_svg::usvg,
};

use crate::subpath::SubPath;

#[derive(Component, TypeUuid, TypePath)]
#[uuid = "45b7766b-c10b-4a90-b16b-e421a1d15598"]
// TODO: remove this
#[allow(dead_code)]
pub struct BezPathGroup {
    pub paths: Vec<BezPath>,
    // TODO: do we really need this? (maybe need for alignment)
    // TODO: but if so, how do we calculate non svg/font data (e.g. custom drawings)
    // pub size: Vec2,
}

impl BezPathGroup {
    // pub fn to_fragment(&self) -> VelloFragment {}

    pub fn from_tree(tree: &usvg::Tree) -> Self {
        let mut bezpaths: Vec<BezPath> = Vec::new();

        for elt in tree.root.descendants() {
            let mut local_path = BezPath::new();

            match &*elt.borrow() {
                usvg::NodeKind::Group(_) => {}
                usvg::NodeKind::Path(path) => {
                    // The semantics of SVG paths don't line up with `BezPath`; we must manually track initial points
                    let mut just_closed = false;
                    let mut most_recent_initial = (0., 0.);
                    for elt in path.data.segments() {
                        match elt {
                            usvg::PathSegment::MoveTo { x, y } => {
                                if std::mem::take(&mut just_closed) {
                                    local_path.move_to(most_recent_initial);
                                }
                                most_recent_initial = (x, y);
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
                }
                usvg::NodeKind::Image(_) => {}
                usvg::NodeKind::Text(_) => {}
            }

            bezpaths.push(local_path);
        }

        BezPathGroup {
            paths: bezpaths,
            // size: Vec2::new(tree.size.width() as f32, tree.size.height() as f32),
        }
    }
}

#[derive(Component)]
pub struct BezPathTracer {
    /// `BezPathGroup` where the original `BezPath` is located at.
    // bezpath_group_handle: Handle<BezPathGroup>,
    /// Index of `BezPath` in the `BezPathGroup`.
    // bezpath_index: usize,
    subpaths: Vec<SubPath>,
}

impl BezPathTracer {
    pub fn new(path: BezPath) -> Self {
        let mut subpaths: Vec<SubPath> = Vec::new();
        let pathels: &[PathEl] = path.elements();

        let mut subpath: SubPath;

        // Initial checking of the first path element.
        // Panic if the first element is not a MoveTo command,
        // return empty if there isn't any element at all.
        if let Some(el) = pathels.first() {
            match el {
                PathEl::MoveTo(p) => subpath = SubPath::new(*p),
                _ => {
                    panic!("First PathEl must be a MoveTo command.");
                }
            }
        } else {
            // Return an empty tracer if the given path is empty.
            return Self { subpaths };
        }

        // Separate given path into multiple subpaths.
        // Skip the first path element as it is being added above during the inital checking.
        for el in pathels.iter().skip(1) {
            match el {
                PathEl::MoveTo(p) => {
                    // MoveTo command represents the start of a new subpath.
                    subpaths.push(subpath);
                    subpath = SubPath::new(*p);
                }
                PathEl::LineTo(p) => subpath.line_to(*p),
                PathEl::QuadTo(p0, p1) => subpath.quad_to(*p0, *p1),
                PathEl::CurveTo(p0, p1, p2) => subpath.curve_to(*p0, *p1, *p2),
                PathEl::ClosePath => subpath.close_path(),
            };
        }

        // Push the last subpath into the vector.
        subpaths.push(subpath);

        Self { subpaths }
    }

    pub fn trace(self, t: f64) -> BezPath {
        let mut bezpath: BezPath = BezPath::new();

        for subpath in self.subpaths {
            subpath.trace(&mut bezpath, t);
        }

        bezpath
    }
}
