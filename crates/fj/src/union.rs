#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::Shape;

/// A union between two shapes
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(C)]
pub struct Union {
    shapes: [Shape; 2],
}

impl Union {
    /// Create a `Union` from two shapes
    pub fn from_shapes(shapes: [Shape; 2]) -> Self {
        Self { shapes }
    }

    /// Access the shapes that make up the union
    pub fn shapes(&self) -> &[Shape; 2] {
        &self.shapes
    }
}

impl From<Union> for Shape {
    fn from(shape: Union) -> Self {
        Self::Union(Box::new(shape))
    }
}
