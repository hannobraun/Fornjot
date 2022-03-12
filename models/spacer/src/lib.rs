use std::collections::HashMap;

#[no_mangle]
pub extern "C" fn model(args: &HashMap<String, String>) -> fj::Shape {
    let outer = args
        .get("outer")
        .unwrap_or(&"1.0".to_owned())
        .parse()
        .unwrap();
    let inner = args
        .get("inner")
        .unwrap_or(&"0.5".to_owned())
        .parse()
        .unwrap();
    let height = args
        .get("height")
        .unwrap_or(&"1.0".to_owned())
        .parse()
        .unwrap();

    let outer_edge = fj::Circle { radius: outer };
    let inner_edge = fj::Circle { radius: inner };

    let footprint = fj::Difference2d {
        a: outer_edge.into(),
        b: inner_edge.into(),
    };

    let spacer = fj::Sweep::from_shape_and_length(footprint.into(), height);

    spacer.into()
}
