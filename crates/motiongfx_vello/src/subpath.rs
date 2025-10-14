use bevy_vello_renderer::vello::kurbo::{BezPath, Point};

#[derive(Clone)]
pub struct SubPath {
    /// Subpath commands.
    paths: Vec<SubPathEl>,
    /// The first MoveTo command in a subpath.
    move_to: Point,
    /// Determines whether this is an closed subpath or not.
    closed: bool,
}

impl SubPath {
    pub fn new(move_to: Point) -> Self {
        Self {
            paths: Vec::new(),
            move_to,
            closed: false,
        }
    }

    // TODO: make it into a range instead
    pub fn trace(self, bezpath: &mut BezPath, t: f64) {
        bezpath.move_to(self.move_to);

        // The last path element is not considered since a segment is made up of at least 2 path elements.
        let count: usize = self.paths.len() - 1;

        let subpath_t: f64 = t * count as f64;
        let subpathel_index: usize = subpath_t as usize;
        // Calcualte the interpolation time for the final path.
        let subpathel_t: f64 = subpath_t - subpathel_index as f64;

        for s in 0..subpathel_index {
            self.paths[s].draw_bezpath(bezpath);
        }

        // When t is or is close to 1.0
        if subpathel_index >= count {
            if self.closed {
                bezpath.close_path();
            }
        } else {
            // Starting point for the trace to occur.
            let start_point: Point;

            if subpathel_index > 0 {
                start_point = self.paths[subpathel_index - 1].end_point()
            } else {
                start_point = self.move_to;
            }

            self.paths[subpathel_index]
                .trace(start_point, subpathel_t)
                .draw_bezpath(bezpath)
        }
    }

    /// Push a "line to" element onto the path.
    ///
    /// start with `move_to`.
    pub fn line_to<P: Into<Point>>(&mut self, p: P) {
        self.paths.push(SubPathEl::LineTo(p.into()));
    }

    /// Push a "quad to" element onto the path.
    ///
    /// start with `move_to`.
    pub fn quad_to<P: Into<Point>>(&mut self, p1: P, p2: P) {
        self.paths.push(SubPathEl::QuadTo(p1.into(), p2.into()));
    }

    /// Push a "curve to" element onto the path.
    ///
    /// start with `move_to`.
    pub fn curve_to<P: Into<Point>>(&mut self, p1: P, p2: P, p3: P) {
        self.paths
            .push(SubPathEl::CurveTo(p1.into(), p2.into(), p3.into()));
    }

    /// Push a "close path" element onto the path.
    ///
    /// start with `move_to`.
    pub fn close_path(&mut self) {
        self.closed = true;
    }
}

#[derive(Clone, Copy)]
pub enum SubPathEl {
    /// Draw a line from the current location to the point.
    LineTo(Point),
    /// Draw a quadratic bezier using the current location and the two points.
    QuadTo(Point, Point),
    /// Draw a cubic bezier using the current location and the three points.
    CurveTo(Point, Point, Point),
}

impl SubPathEl {
    /// Trace from a `Point` to a `SubPathEl`.
    pub fn trace(self, a: Point, t: f64) -> Self {
        let trace_path: Self;

        match self {
            SubPathEl::LineTo(b) => trace_path = SubPathEl::LineTo(Point::lerp(a, b, t)),
            SubPathEl::QuadTo(b, c) => {
                let ab: Point = Point::lerp(a, b, t);
                let bc: Point = Point::lerp(b, c, t);

                let abc: Point = Point::lerp(ab, bc, t);

                trace_path = SubPathEl::QuadTo(ab, abc);
            }
            SubPathEl::CurveTo(b, c, d) => {
                let ab = Point::lerp(a, b, t);
                let bc = Point::lerp(b, c, t);
                let cd = Point::lerp(c, d, t);

                let abc = Point::lerp(ab, bc, t);
                let bcd = Point::lerp(bc, cd, t);

                let abcd = Point::lerp(abc, bcd, t);

                trace_path = SubPathEl::CurveTo(ab, abc, abcd);
            }
        }

        trace_path
    }

    /// Adds its own command to the `BezPath`.
    pub fn draw_bezpath(self, bezpath: &mut BezPath) {
        match self {
            SubPathEl::LineTo(p) => bezpath.line_to(p),
            SubPathEl::QuadTo(p0, p1) => bezpath.quad_to(p0, p1),
            SubPathEl::CurveTo(p0, p1, p2) => bezpath.curve_to(p0, p1, p2),
        }
    }

    /// Is this path element finite?
    #[inline]
    pub fn is_finite(&self) -> bool {
        match self {
            SubPathEl::LineTo(p) => p.is_finite(),
            SubPathEl::QuadTo(p, p2) => p.is_finite() && p2.is_finite(),
            SubPathEl::CurveTo(p, p2, p3) => p.is_finite() && p2.is_finite() && p3.is_finite(),
        }
    }

    /// Is this path element NaN?
    #[inline]
    pub fn is_nan(&self) -> bool {
        match self {
            SubPathEl::LineTo(p) => p.is_nan(),
            SubPathEl::QuadTo(p, p2) => p.is_nan() || p2.is_nan(),
            SubPathEl::CurveTo(p, p2, p3) => p.is_nan() || p2.is_nan() || p3.is_nan(),
        }
    }

    /// Get the end point of the path element
    pub fn end_point(&self) -> Point {
        match self {
            SubPathEl::LineTo(p1) => *p1,
            SubPathEl::QuadTo(_, p2) => *p2,
            SubPathEl::CurveTo(_, _, p3) => *p3,
        }
    }
}
