use std::collections::BTreeSet;

use crate::{
    local::Local,
    objects::{Edge, GlobalVertex, Solid, Vertex, VerticesOfEdge},
};

use super::intersection::{
    CurveFaceIntersectionList, SurfaceSurfaceIntersection,
};

/// Computes the shape that is the union of the two provided shapes
pub fn union(a: impl Into<Solid>, b: impl Into<Solid>) -> Solid {
    // TASK: Implement algorithm from "Boundary Representation Modelling
    //       Techniques", section 6.1.1 (pages 127 ff.).

    let a = a.into();
    let b = b.into();

    let mut faces = BTreeSet::new();

    // Check the faces of both shapes for intersections.
    for face_a in a.faces() {
        for face_b in b.faces() {
            // First step in determining the intersections between two faces:
            // Determine the intersection of their surfaces.
            let intersection = SurfaceSurfaceIntersection::compute(
                face_a.surface(),
                face_b.surface(),
            );

            let intersection = match intersection {
                Some(intersection) => intersection,
                None => {
                    // We're not getting an intersection curve, which means the
                    // surfaces either don't intersect (which can only mean
                    // they're parallel), or they are coincident.

                    // TASK: Handle surfaces being coincident.
                    continue;
                }
            };

            let [curve_a, curve_b] = intersection.local_intersection_curves;

            // Check the curve's intersections against the faces. The result of
            // this operation are the intersections between the faces.
            let intersections_a =
                CurveFaceIntersectionList::compute(&curve_a, face_a);
            let intersections_b =
                CurveFaceIntersectionList::compute(&curve_b, face_b);

            // Depending on which of the faces the two surface's intersection
            // curve intersects with, we can draw conclusions about which of
            // them we need to keep.
            //
            // If exactly one of the faces intersects, we know that this face is
            // part of the boundary of the union. We add it to `faces`.
            //
            // If we're not adding a face here, that is not a definite decision.
            // The face might still be added to `faces` in another loop
            // iteration, as part of a comparison with another face.

            match (intersections_a.is_empty(), intersections_b.is_empty()) {
                // Both faces intersect the intersection curve, which means they
                // intersect each other.
                (false, false) => {
                    // Nothing to do here. We'll resume after the `match`.
                }

                // Intersection curve intersects only one of the faces.
                //
                // TASK: This doesn't mean that that face should go into the
                //       union as-is. It means that some version of the face
                //       goes into the union, but if there's a proper
                //       intersection with another face, the face needs to be
                //       cut to size.
                //
                //       Add it to a list of candidates that still can be
                //       modified instead.
                (false, true) => {
                    faces.insert(face_a.clone());
                    continue;
                }
                (true, false) => {
                    faces.insert(face_b.clone());
                    continue;
                }

                // Intersection curve intersects none of the faces. This can
                // only happen if none of the faces is contained in a shell that
                // the other face is a part of. They must be part of distinct
                // solids, and both need to end up in the union.
                (true, true) => {
                    faces.insert(face_a.clone());
                    faces.insert(face_b.clone());
                    continue;
                }
            }

            // If we reach this point, we have two faces that intersect each
            // other. Merge the intersection lists, then update the faces as
            // appropriate.
            let intersections = intersections_a.merge(&intersections_b);

            // TASK: For a start, we can ignore all those tasks here, and try to
            //       get the expected interior cycle in there. Then worry about
            //       making the diff between the actual and expected shapes
            //       smaller, as a next step.
            for face in [face_a, face_b] {
                // TASK: Handle the case where the intersections don't cross any
                //       edges of the face. This means, the intersection is part
                //       of an interior cycle.
                //
                //       At least in some cases. The problem here is that I
                //       can't really tell per-edge. An edge might be on the
                //       inside of the face, so it might *look* like it should
                //       be part of an interior cycle, but then other
                //       intersections connect that edge to the outside.
                // TASK: Handle the case where the intersections *do* cross
                //       edges of the face. This must result in these edges
                //       being shortened.

                // TASK: I probably need to move this piece of code outside of
                //       the inner loop. I think we need to store the
                //       intersections per-face, then merge *all* intersections
                //       of a face, in some way, to cut the face to size.

                // TASK: Implement.
                let _ = face;
            }

            for [start, end] in intersections {
                let curve = intersection.global_intersection_curve;

                // TASK: This conversion isn't right. It can result in slightly
                //       different global vertices for different edges, even
                //       though those might be where those edges connect, and
                //       thus supposed to be exactly the same.
                let [start_global, end_global] = [start, end].map(|point| {
                    let position = curve.point_from_curve_coords(point);
                    GlobalVertex::from_position(position)
                });

                let vertices = VerticesOfEdge::from_vertices([
                    Vertex::new(start, start_global),
                    Vertex::new(end, end_global),
                ]);

                let edge_a = Edge::new(Local::new(curve_a, curve), vertices);
                let edge_b = Edge::new(Local::new(curve_b, curve), vertices);

                // TASK: Even if the conversion to `GlobalVertex` above weren't
                //       a problem, what would I even do with these edges I
                //       created? They need to end up in cycles, but at this
                //       point, I have no way of knowing which cycles those are
                //       going to be, and in what order the edges need to go in
                //       there.

                // TASK: Implement.
                let _ = edge_a;
                let _ = edge_b;
            }
        }
    }

    Solid::new().with_faces(faces)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        algorithms::{union, TransformObject},
        objects::{Cycle, Face, Solid},
    };

    #[test]
    fn distinct() {
        let a = Solid::build()
            .cube_from_edge_length(1.)
            .translate([-1., -1., -1.]);
        let b = Solid::build()
            .cube_from_edge_length(1.)
            .translate([1., 1., 1.]);

        let mut all_faces = Vec::new();
        all_faces.extend(a.faces().cloned());
        all_faces.extend(b.faces().cloned());

        let union = union(a, b);

        assert_eq!(union, Solid::new().with_faces(all_faces));
    }

    #[test]
    fn a_contains_b() {
        let a = Solid::build().cube_from_edge_length(2.);
        let b = Solid::build().cube_from_edge_length(1.);

        let union = union(a.clone(), b);

        assert_eq!(union, a.into_solid());
    }

    #[test]
    fn b_contains_a() {
        let a = Solid::build().cube_from_edge_length(1.);
        let b = Solid::build().cube_from_edge_length(2.);

        let union = union(a, b.clone());

        assert_eq!(union, b.into_solid());
    }

    #[test]
    fn intersecting_with_broken_edges_in_a() {
        let a = Solid::build()
            .cube_from_edge_length(1.)
            .translate([0., 0., 1.]);
        let b = Solid::build().cube_from_edge_length(2.);

        let union = union(a.clone(), b.clone());

        let expected = {
            // The face where the two cubes connect.
            let connecting_face = b.top_face().clone();

            // Build the smaller cube that attaches to the larger cube. Since
            // `a` intersects with `b`, we need to cut it down here.
            let smaller_cube = {
                let left = a.left_face().clone();
                let right = a.right_face().clone();
                let front = a.front_face().clone();
                let back = a.back_face().clone();

                // Let's create some shorthands for values we're going to need a
                // lot.
                let x = 0.5; // half the width of the smaller cube
                let y = 0.25; // half the height of the smaller cube

                let updated_faces =
                    [&left, &right, &front, &back].into_iter().map(|face| {
                        (
                            face,
                            Face::build(*face.surface())
                                .polygon_from_points([
                                    [-x, -y],
                                    [x, -y],
                                    [x, y],
                                    [-x, y],
                                ])
                                .into_face(),
                        )
                    });

                let mut smaller_cube = a.into_solid();
                for (original, updated) in updated_faces {
                    smaller_cube =
                        smaller_cube.update_face(original, |_| updated);
                }

                smaller_cube
            };

            // Now that we have the smaller cube, we can connect it to the
            // larger one.
            b.into_solid()
                .update_face(&connecting_face, |face| {
                    // Add a polygon to the connecting face, where the smaller
                    // cube attaches.
                    let polygon = Cycle::build(*face.surface())
                        .polygon_from_points([
                            [0.5, 0.5],
                            [1.5, 0.5],
                            [1.5, 1.5],
                            [0.5, 1.5],
                        ]);
                    face.clone().with_interiors([polygon])
                })
                .with_faces(smaller_cube.into_faces())
        };
        assert_eq!(union, expected);
    }

    // TASK: intersection, broken edges in b
}
