//! # Fornjot Modeling Library
//!
//! This library is part of the [Fornjot] ecosystem. Fornjot is an open-source,
//! code-first CAD application; and collection of libraries that make up the CAD
//! application, but can be used independently.
//!
//! The purpose of this library is to support Fornjot models, which are just
//! Rust libraries. Models depend on this library and use the primitives defined
//! here to define a CAD model. Together with the Fornjot application, this
//! library forms the part of Fornjot that is relevant to end users.
//!
//! To display the created CAD model, or export it to another file format, you
//! need the Fornjot application. Please refer to the [Fornjot repository] for
//! usage examples.
//!
//! [Fornjot]: https://www.fornjot.app/
//! [Fornjot repository]: https://github.com/hannobraun/Fornjot

#![warn(missing_docs)]

pub mod syntax;

#[doc(hidden)]
pub mod abi;
mod angle;
mod context;
mod group;
mod host;
mod metadata;
mod model;
mod shape_2d;
mod sweep;
mod transform;

pub use self::{
    angle::*,
    context::{
        Context, ContextError, ContextExt, MissingArgument, ParseFailed,
    },
    group::Group,
    host::{Host, HostExt},
    metadata::{ArgumentMetadata, Metadata, ModelMetadata},
    model::Model,
    shape_2d::*,
    sweep::Sweep,
    transform::Transform,
};
pub use fj_proc::*;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A shape
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(C)]
#[allow(improper_ctypes)] // Box isn't FFI-safe
pub enum Shape {
    /// A group of two 3-dimensional shapes
    Group(Box<Group>),

    /// A 2D shape
    Shape2d(Shape2d),

    /// A sweep of 2-dimensional shape along the z-axis
    Sweep(Sweep),

    /// A transformed 3-dimensional shape
    Transform(Box<Transform>),
}
