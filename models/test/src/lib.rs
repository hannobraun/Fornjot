use std::f64::consts::PI;

use fj::{syntax::*, Angle, HostExt, Metadata, ModelMetadata};

fj::register_model!(|host| {
    host.register_model(Test);

    Ok(
        Metadata::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .with_short_description(env!("CARGO_PKG_DESCRIPTION"))
            .with_description(include_str!("../README.md"))
            .with_homepage(env!("CARGO_PKG_HOMEPAGE"))
            .with_repository(env!("CARGO_PKG_REPOSITORY"))
            .with_license(env!("CARGO_PKG_LICENSE")),
    )
});

struct Test;

impl fj::Model for Test {
    fn shape(
        &self,
        _ctx: &dyn fj::Context,
    ) -> Result<fj::Shape, Box<dyn std::error::Error + Send + Sync>> {
        let a = star(4, 1., [0, 255, 0, 200]);
        let b = star(5, -1., [255, 0, 0, 255])
            .rotate([1., 1., 1.], Angle::from_deg(45.))
            .translate([3., 3., 1.]);
        let c = spacer().translate([6., 6., 1.]);

        let group = a.group(&b).group(&c);

        Ok(group.into())
    }

    fn metadata(&self) -> ModelMetadata {
        ModelMetadata::new("Test")
    }
}

fn star(num_points: u64, height: f64, color: [u8; 4]) -> fj::Shape {
    let r1 = 1.;
    let r2 = 2.;

    // We need to figure out where to generate vertices, depending on the number
    // of points the star is supposed to have. Let's generate an iterator that
    // gives us the angle and radius for each vertex.
    let num_vertices = num_points * 2;
    let vertex_iter = (0..num_vertices).map(|i| {
        let angle = Angle::from_rad(2. * PI / num_vertices as f64 * i as f64);
        let radius = if i % 2 == 0 { r1 } else { r2 };
        (angle, radius)
    });

    // Now that we got that iterator prepared, generating the vertices is just a
    // bit of trigonometry.
    let mut outer = Vec::new();
    let mut inner = Vec::new();
    for (angle, radius) in vertex_iter {
        let (sin, cos) = angle.rad().sin_cos();

        let x = cos * radius;
        let y = sin * radius;

        outer.push([x, y]);
        inner.push([x / 2., y / 2.]);
    }

    let outer = fj::Sketch::from_points(outer).with_color(color);
    let inner = fj::Sketch::from_points(inner);

    let footprint = fj::Difference2d::from_shapes([outer.into(), inner.into()]);

    let star = fj::Sweep::from_path(footprint.into(), [0., 0., height]);

    star.into()
}

fn spacer() -> fj::Shape {
    let outer = 2.;
    let inner = 1.;
    let height = 2.;

    let outer_edge = fj::Sketch::from_circle(fj::Circle::from_radius(outer))
        .with_color([0, 0, 255, 255]);
    let inner_edge = fj::Sketch::from_circle(fj::Circle::from_radius(inner));

    let footprint = outer_edge.difference(&inner_edge);
    let spacer = footprint.sweep([0., 0., height]);

    spacer.into()
}
