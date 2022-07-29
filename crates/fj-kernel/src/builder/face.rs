use std::ops::Deref;

use fj_math::Point;

use crate::objects::{Cycle, Face, Surface};

/// API for building a [`Face`]
pub struct FaceBuilder {
    surface: Surface,
}

impl FaceBuilder {
    /// Construct an instance of `FaceBuilder`
    ///
    /// Also see [`Face::build`].
    pub fn new(surface: Surface) -> Self {
        Self { surface }
    }

    /// Construct a polygon from a list of points
    pub fn polygon_from_points(
        &self,
        points: impl IntoIterator<Item = impl Into<Point<2>>>,
    ) -> FacePolygon {
        let face = Face::new(self.surface)
            .with_exteriors([
                Cycle::build(self.surface).polygon_from_points(points)
            ]);

        FacePolygon { face }
    }
}

/// A polygon
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct FacePolygon {
    face: Face,
}

impl FacePolygon {
    /// Add a hole to the polygon
    pub fn with_hole(
        mut self,
        points: impl IntoIterator<Item = impl Into<Point<2>>>,
    ) -> Self {
        let surface = *self.face.surface();
        self.face = self.face.with_interiors([
            Cycle::build(surface).polygon_from_points(points)
        ]);

        self
    }

    /// Consume the `Polygon` and return the [`Face`] it wraps
    pub fn into_face(self) -> Face {
        self.face
    }
}

impl Deref for FacePolygon {
    type Target = Face;

    fn deref(&self) -> &Self::Target {
        &self.face
    }
}

impl From<FacePolygon> for Face {
    fn from(polygon: FacePolygon) -> Self {
        polygon.into_face()
    }
}
