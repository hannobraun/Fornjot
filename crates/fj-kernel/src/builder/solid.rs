use std::ops::Deref;

use fj_math::Scalar;

use crate::{
    algorithms::TransformObject,
    objects::{Face, Solid, Surface},
};

/// API for building a [`Solid`]
pub struct SolidBuilder;

impl SolidBuilder {
    /// Create a cube from the length of its edges
    pub fn cube_from_edge_length(
        &self,
        edge_length: impl Into<Scalar>,
    ) -> Cube {
        // Let's define short-hands for some values. We're going to need them a
        // lot.
        let h = edge_length.into() / 2.; // half the edge length
        const Z: Scalar = Scalar::ZERO;

        let points = [[-h, -h], [h, -h], [h, h], [-h, h]];

        let planes = [
            Surface::xy_plane().translate([Z, Z, -h]), // bottom
            Surface::xy_plane().translate([Z, Z, h]),  // top
            Surface::xz_plane().translate([Z, -h, Z]), // front
            Surface::xz_plane().translate([Z, h, Z]),  // back
            Surface::yz_plane().translate([-h, Z, Z]), // left
            Surface::yz_plane().translate([h, Z, Z]),  // right
        ];

        let faces =
            planes.map(|plane| Face::build(plane).polygon_from_points(points));
        let [left_face, right_face, front_face, back_face, bottom_face, top_face] =
            faces.clone().map(Into::into);

        let solid = Solid::new().with_faces(faces);

        Cube {
            solid,
            left_face,
            right_face,
            front_face,
            back_face,
            bottom_face,
            top_face,
        }
    }
}

/// A cube
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Cube {
    solid: Solid,

    left_face: Face,
    right_face: Face,
    front_face: Face,
    back_face: Face,
    bottom_face: Face,
    top_face: Face,
}

impl Cube {
    /// Access the left face of the cube
    pub fn left_face(&self) -> &Face {
        &self.left_face
    }

    /// Access the right face of the cube
    pub fn right_face(&self) -> &Face {
        &self.right_face
    }

    /// Access the front face of the cube
    pub fn front_face(&self) -> &Face {
        &self.front_face
    }

    /// Access the back face of the cube
    pub fn back_face(&self) -> &Face {
        &self.back_face
    }

    /// Access the bottom face of the cube
    pub fn bottom_face(&self) -> &Face {
        &self.bottom_face
    }

    /// Access the top face of the cube
    pub fn top_face(&self) -> &Face {
        &self.top_face
    }

    /// Consume the cube and return the [`Solid`] it wraps
    pub fn into_solid(self) -> Solid {
        self.solid
    }
}

impl Deref for Cube {
    type Target = Solid;

    fn deref(&self) -> &Self::Target {
        &self.solid
    }
}

impl From<Cube> for Solid {
    fn from(cube: Cube) -> Self {
        cube.into_solid()
    }
}

impl TransformObject for Cube {
    fn transform(mut self, transform: &fj_math::Transform) -> Self {
        self.solid = self.solid.transform(transform);

        self.left_face = self.left_face.transform(transform);
        self.right_face = self.right_face.transform(transform);
        self.front_face = self.front_face.transform(transform);
        self.back_face = self.back_face.transform(transform);
        self.bottom_face = self.bottom_face.transform(transform);
        self.top_face = self.top_face.transform(transform);

        self
    }
}
