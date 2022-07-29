//! Intersection algorithms

mod curve_edge;
mod curve_face;
mod face_face;
mod line_segment;
mod surface_surface;

pub use self::{
    curve_edge::CurveEdgeIntersection,
    curve_face::{CurveFaceIntersection, CurveFaceIntersectionList},
    face_face::FaceFaceIntersection,
    line_segment::LineSegmentIntersection,
    surface_surface::SurfaceSurfaceIntersection,
};
