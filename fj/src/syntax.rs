pub trait Rotate {
    /// Create a rotation
    ///
    /// Create a rotation that rotates `shape` by `angle` around an axis defined
    /// by `axis`.
    fn rotate(&self, axis: [f64; 3], angle: f64) -> crate::Transform;
}

impl<T> Rotate for T
where
    T: Clone + Into<crate::Shape3d>,
{
    fn rotate(&self, axis: [f64; 3], angle: f64) -> crate::Transform {
        let shape = self.clone().into();
        crate::Transform {
            shape,
            axis,
            angle,
            offset: [0.; 3],
        }
    }
}

pub trait Sketch {
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

pub trait Sweep {
    fn sweep(&self, length: f64) -> crate::Sweep;
}

impl<T> Sweep for T
where
    T: Clone + Into<crate::Shape2d>,
{
    fn sweep(&self, length: f64) -> crate::Sweep {
        let shape = self.clone().into();
        crate::Sweep::from_shape_and_length(shape, length)
    }
}

pub trait Translate {
    /// Create a translation
    ///
    /// Create a translation that translates `shape` by `offset`.
    fn translate(&self, offset: [f64; 3]) -> crate::Transform;
}

impl<T> Translate for T
where
    T: Clone + Into<crate::Shape3d>,
{
    fn translate(&self, offset: [f64; 3]) -> crate::Transform {
        let shape = self.clone().into();
        crate::Transform {
            shape,
            axis: [1., 0., 0.],
            angle: 0.,
            offset,
        }
    }
}

pub trait Union {
    fn union<Other>(&self, other: &Other) -> crate::Union
    where
        Other: Clone + Into<crate::Shape3d>;
}

impl<T> Union for T
where
    T: Clone + Into<crate::Shape3d>,
{
    fn union<Other>(&self, other: &Other) -> crate::Union
    where
        Other: Clone + Into<crate::Shape3d>,
    {
        let a = self.clone().into();
        let b = other.clone().into();

        crate::Union { a, b }
    }
}
