use std::collections::HashMap;

use fj::syntax::*;

#[no_mangle]
pub extern "C" fn model(_: &HashMap<String, String>) -> fj::Shape {
    #[rustfmt::skip]
    let points = vec![
        [-0.5, -0.5],
        [ 0.5, -0.5],
        [ 0.5,  0.5],
        [-0.5,  0.5],
    ];

    let a = fj::Sketch::from_points(points).sweep([0., 0., 1.]);
    let b = a.translate([0.5, 0.5, 0.5]);

    let union = a.union(&b);

    union.into()
}
