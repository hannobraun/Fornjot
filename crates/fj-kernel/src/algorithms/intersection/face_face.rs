use fj_math::Point;

use crate::objects::{Curve, Face};

/// An intersection between two faces
pub struct FaceFaceIntersection {
    /// The intersection curves, in surface coordinates of the respective face
    pub local_intersection_curves: [Curve<2>; 2],

    /// The intersection curve, in global coordinates
    pub global_intersection_curve: Curve<3>,

    /// The interval of this intersection, in curve coordinates
    pub intersection_interval: [Point<1>; 2],
}

impl FaceFaceIntersection {
    /// Compute the intersections between two faces
    pub fn compute(
        face_a: &Face,
        face_b: &Face,
    ) -> impl Iterator<Item = FaceFaceIntersection> {
        // TASK: Implement.
        let _ = face_a;
        let _ = face_b;
        std::iter::empty()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn fail() {
        panic!()
    }
}
