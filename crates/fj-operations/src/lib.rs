//! # Fornjot CAD Operations
//!
//! This library is part of the [Fornjot] ecosystem. Fornjot is an open-source,
//! code-first CAD application; and collection of libraries that make up the CAD
//! application, but can be used independently.
//!
//! This library is an internal component of Fornjot. It is not relevant to end
//! users that just want to create CAD models.
//!
//! Fornjot models use the [`fj`] crate to define a shape. This crate provides
//! the connection between [`fj`] and the Fornjot kernel. It translates those
//! operations into terms the kernel can understand.
//!
//! [Fornjot]: https://www.fornjot.app/
//! [`fj`]: https://crates.io/crates/fj

#![warn(missing_docs)]

pub mod shape_processor;

mod difference_2d;
mod group;
mod sketch;
mod sweep;
mod transform;
mod union;

use fj_interop::debug::DebugInfo;
use fj_kernel::{
    algorithms::Tolerance,
    objects::{Face, Sketch},
    validation::{validate, Validated, ValidationConfig, ValidationError},
};
use fj_math::Aabb;

/// Implemented for all operations from the [`fj`] crate
pub trait Shape {
    /// The type that is used for the shape's boundary representation
    type Brep;

    /// Compute the boundary representation of the shape
    fn compute_brep(
        &self,
        config: &ValidationConfig,
        tolerance: Tolerance,
        debug_info: &mut DebugInfo,
    ) -> Result<Validated<Self::Brep>, ValidationError>;

    /// Access the axis-aligned bounding box of a shape
    ///
    /// If a shape is empty, its [`Aabb`]'s `min` and `max` points must be equal
    /// (but are otherwise not specified).
    fn bounding_volume(&self) -> Aabb<3>;
}

impl Shape for fj::Shape {
    type Brep = Vec<Face>;

    fn compute_brep(
        &self,
        config: &ValidationConfig,
        tolerance: Tolerance,
        debug_info: &mut DebugInfo,
    ) -> Result<Validated<Self::Brep>, ValidationError> {
        match self {
            Self::Shape2d(shape) => validate(
                shape
                    .compute_brep(config, tolerance, debug_info)?
                    .into_inner()
                    .into_faces()
                    .collect(),
                config,
            ),
            Self::Group(shape) => {
                shape.compute_brep(config, tolerance, debug_info)
            }
            Self::Sweep(shape) => validate(
                shape
                    .compute_brep(config, tolerance, debug_info)?
                    .into_inner()
                    .into_faces()
                    .collect(),
                config,
            ),
            Self::Transform(shape) => {
                shape.compute_brep(config, tolerance, debug_info)
            }
            Self::Union(shape) => validate(
                shape
                    .compute_brep(config, tolerance, debug_info)?
                    .into_inner()
                    .into_faces()
                    .into_iter()
                    .collect(),
                config,
            ),
        }
    }

    fn bounding_volume(&self) -> Aabb<3> {
        match self {
            Self::Shape2d(shape) => shape.bounding_volume(),
            Self::Group(shape) => shape.bounding_volume(),
            Self::Sweep(shape) => shape.bounding_volume(),
            Self::Transform(shape) => shape.bounding_volume(),
            Self::Union(shape) => shape.bounding_volume(),
        }
    }
}

impl Shape for fj::Shape2d {
    type Brep = Sketch;

    fn compute_brep(
        &self,
        config: &ValidationConfig,
        tolerance: Tolerance,
        debug_info: &mut DebugInfo,
    ) -> Result<Validated<Self::Brep>, ValidationError> {
        match self {
            Self::Difference(shape) => {
                shape.compute_brep(config, tolerance, debug_info)
            }
            Self::Sketch(shape) => {
                shape.compute_brep(config, tolerance, debug_info)
            }
        }
    }

    fn bounding_volume(&self) -> Aabb<3> {
        match self {
            Self::Difference(shape) => shape.bounding_volume(),
            Self::Sketch(shape) => shape.bounding_volume(),
        }
    }
}
