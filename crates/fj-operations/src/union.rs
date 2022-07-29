use fj_interop::debug::DebugInfo;
use fj_kernel::{
    algorithms::{union, Tolerance},
    objects::Solid,
    validation::{validate, Validated, ValidationConfig, ValidationError},
};
use fj_math::Aabb;

use super::Shape;

impl Shape for fj::Union {
    type Brep = Solid;

    fn compute_brep(
        &self,
        config: &ValidationConfig,
        tolerance: Tolerance,
        debug_info: &mut DebugInfo,
    ) -> Result<Validated<Self::Brep>, ValidationError> {
        // Can be cleaned up, once `each_ref` is stable:
        // https://doc.rust-lang.org/std/primitive.array.html#method.each_ref
        let [a, b] = self.shapes();
        let shapes_ref = [a, b];

        // Can be cleaned up, once `try_map` is stable:
        // https://doc.rust-lang.org/std/primitive.array.html#method.try_map
        let [a, b] = shapes_ref
            .map(|shape| shape.compute_brep(config, tolerance, debug_info));
        let shapes = [a?, b?];
        let [a, b] = shapes.map(|shape| shape.into_inner());

        let a = Solid::from_faces(a);
        let b = Solid::from_faces(b);

        let union = union(a, b);
        validate(union, config)
    }

    fn bounding_volume(&self) -> Aabb<3> {
        // This is a conservative estimate of the bounding box: It's never going
        // to be bigger than the bounding box of the original shape that another
        // is being subtracted from.
        self.shapes()[0].bounding_volume()
    }
}
