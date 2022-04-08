use std::collections::{BTreeSet, HashSet};

use fj_debug::{DebugInfo, TriangleEdgeCheck};
use fj_math::{Point, Scalar, Segment};
use parry2d_f64::query::{Ray as Ray2, RayCast as _};
use parry3d_f64::query::Ray as Ray3;

use crate::{
    algorithms::CycleApprox,
    geometry::{self, Surface},
};

pub struct Polygon {
    surface: Surface,
    segments: HashSet<[geometry::Point<2>; 2]>,
}

impl Polygon {
    pub fn new(
        exterior: CycleApprox,
        interiors: impl IntoIterator<Item = CycleApprox>,
        surface: Surface,
    ) -> Self {
        let segments = exterior
            .segments()
            .into_iter()
            .chain(
                interiors
                    .into_iter()
                    .flat_map(|cycle_approx| cycle_approx.segments()),
            )
            .into_iter()
            .map(|segment| {
                segment.points().map(|point| {
                    // Can't panic, unless the approximation wrongfully
                    // generates points that are not in the surface.
                    surface.point_model_to_surface(point)
                })
            })
            .collect();

        Self { surface, segments }
    }

    pub fn contains_triangle(
        &self,
        &[a, b, c]: &[geometry::Point<2>; 3],
        outside: Point<2>,
        debug_info: &mut DebugInfo,
    ) -> bool {
        for segment in [a, b, c, a].windows(2) {
            // This can't panic, as we passed `2` to `windows`. It can be
            // cleaned up a bit, once `array_windows` is stable.
            let segment = [segment[0], segment[1]];

            // If the segment is an edge of the face, we don't need to take a
            // closer look.
            if self.contains_segment(&segment) {
                continue;
            }

            // To determine if the edge is within the polygon, we determine if
            // its center point is in the polygon.
            let center = segment[0] + (segment[1] - segment[0]) / Scalar::TWO;

            let origin = center;
            let dir = outside - center;
            let ray = Ray2 {
                origin: origin.to_na(),
                dir: dir.to_na(),
            };

            let mut check = TriangleEdgeCheck::new(Ray3 {
                origin: self.surface.point_surface_to_model(&origin).to_na(),
                dir: self.surface.vector_surface_to_model(&dir).to_na(),
            });

            // We need to keep track of where our ray hits the edges. Otherwise,
            // if the ray hits a vertex, we might count that hit twice, as every
            // vertex is attached to two edges.
            let mut hits = BTreeSet::new();

            // Use ray-casting to determine if `center` is within the
            // face-polygon.
            for edge in &self.segments {
                // Please note that we if we get to this point, then the point
                // is not on a polygon edge, due to the check above. We don't
                // need to handle any edge cases that would arise from that
                // case.

                let edge = Segment::from(edge.map(|point| point.native()));

                let intersection = edge
                    .to_parry()
                    .cast_local_ray(&ray, f64::INFINITY, true)
                    .map(Scalar::from_f64);

                if let Some(t) = intersection {
                    // Due to slight inaccuracies, we might get different values
                    // for the same intersections. Let's round `t` before using
                    // it.
                    let eps = 1_000_000.0;
                    let t = (t * eps).round() / eps;

                    if hits.insert(t) {
                        check.hits.push(t.into_f64());
                    }
                }
            }

            debug_info.triangle_edge_checks.push(check);

            if hits.len() % 2 == 0 {
                // The segment is outside of the face. This means we can throw
                // away the whole triangle.
                return false;
            }
        }

        // If we didn't throw away the triangle up till now, this means all its
        // edges are within the face.
        true
    }

    pub fn contains_segment(&self, &[a, b]: &[geometry::Point<2>; 2]) -> bool {
        self.segments.contains(&[a, b]) || self.segments.contains(&[b, a])
    }
}