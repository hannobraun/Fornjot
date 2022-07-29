//! Convenient syntax for `fj` operations
//!
//! This model defines extension traits, which provide convenient syntax for
//! the various operations defined in this trait.

/// Convenient syntax to create an [`fj::Difference2d`]
///
/// [`fj::Difference2d`]: crate::Difference2d
pub trait Difference {
    /// Create a difference between `self` and `other`
    fn difference<Other>(&self, other: &Other) -> crate::Difference2d
    where
        Other: Clone + Into<crate::Shape2d>;
}

impl<T> Difference for T
where
    T: Clone + Into<crate::Shape2d>,
{
    fn difference<Other>(&self, other: &Other) -> crate::Difference2d
    where
        Other: Clone + Into<crate::Shape2d>,
    {
        let a = self.clone().into();
        let b = other.clone().into();

        crate::Difference2d::from_shapes([a, b])
    }
}

/// Convenient syntax to create an [`fj::Group`]
///
/// [`fj::Group`]: crate::Group
pub trait Group {
    /// Create a group with `self` and `other`
    fn group<Other>(&self, other: &Other) -> crate::Group
    where
        Other: Clone + Into<crate::Shape>;
}

impl<T> Group for T
where
    T: Clone + Into<crate::Shape>,
{
    fn group<Other>(&self, other: &Other) -> crate::Group
    where
        Other: Clone + Into<crate::Shape>,
    {
        let a = self.clone().into();
        let b = other.clone().into();

        crate::Group { a, b }
    }
}

/// Convenient syntax to create an [`fj::Sketch`]
///
/// [`fj::Sketch`]: crate::Sketch
pub trait Sketch {
    /// Create a sketch from `self`
    ///
    /// Can be called on any type that implements `AsRef<[[f64; 2]]`, which is
    /// implemented for types like slices, arrays, or `Vec`.
    fn sketch(&self) -> crate::Sketch;
}

impl<T> Sketch for T
where
    T: AsRef<[[f64; 2]]>,
{
    fn sketch(&self) -> crate::Sketch {
        crate::Sketch::from_points(self.as_ref().to_vec())
    }
}

/// Convenient syntax to create an [`fj::Sweep`]
///
/// [`fj::Sweep`]: crate::Sweep
pub trait Sweep {
    /// Sweep `self` along a straight path
    fn sweep(&self, path: [f64; 3]) -> crate::Sweep;
}

impl<T> Sweep for T
where
    T: Clone + Into<crate::Shape2d>,
{
    fn sweep(&self, path: [f64; 3]) -> crate::Sweep {
        let shape = self.clone().into();
        crate::Sweep::from_path(shape, path)
    }
}

/// Convenient syntax to create an [`fj::Transform`]
///
/// [`fj::Transform`]: crate::Transform
pub trait Transform {
    /// Create a rotation
    ///
    /// Create a rotation that rotates `shape` by `angle` around an axis defined
    /// by `axis`.
    fn rotate(&self, axis: [f64; 3], angle: crate::Angle) -> crate::Transform;

    /// Create a translation
    ///
    /// Create a translation that translates `shape` by `offset`.
    fn translate(&self, offset: [f64; 3]) -> crate::Transform;
}

impl<T> Transform for T
where
    T: Clone + Into<crate::Shape>,
{
    fn rotate(&self, axis: [f64; 3], angle: crate::Angle) -> crate::Transform {
        let shape = self.clone().into();
        crate::Transform {
            shape,
            axis,
            angle,
            offset: [0.; 3],
        }
    }

    fn translate(&self, offset: [f64; 3]) -> crate::Transform {
        let shape = self.clone().into();
        crate::Transform {
            shape,
            axis: [1., 0., 0.],
            angle: crate::Angle::from_rad(0.),
            offset,
        }
    }
}

/// Convenient syntax to create an [`fj::Union`]
///
/// [`fj::Union`]: crate::Union
pub trait Union {
    /// Create a difference between `self` and `other`
    fn union<Other>(&self, other: &Other) -> crate::Union
    where
        Other: Clone + Into<crate::Shape>;
}

impl<T> Union for T
where
    T: Clone + Into<crate::Shape>,
{
    fn union<Other>(&self, other: &Other) -> crate::Union
    where
        Other: Clone + Into<crate::Shape>,
    {
        let a = self.clone().into();
        let b = other.clone().into();

        crate::Union::from_shapes([a, b])
    }
}
