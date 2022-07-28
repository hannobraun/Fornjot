use std::f64::consts::PI;

use fj::{
    ArgumentMetadata, ContextExt, HostExt, Metadata, Model, ModelMetadata,
};

fj::register_model!(|host| {
    host.register_model(Star);

    Ok(Metadata::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    ))
});

struct Star;

impl Model for Star {
    fn shape(
        &self,
        ctx: &dyn fj::Context,
    ) -> Result<fj::Shape, Box<dyn std::error::Error + Send + Sync>> {
        let num_points: u64 =
            ctx.parse_optional_argument("num_points")?.unwrap_or(5);
        let r1: f64 = ctx.parse_optional_argument("r1")?.unwrap_or(1.0);
        let r2: f64 = ctx.parse_optional_argument("r2")?.unwrap_or(2.0);
        let h: f64 = ctx.parse_optional_argument("h")?.unwrap_or(1.0);

        if num_points < 3 {
            todo!();
        }
        if r1 < 1.0 {
            todo!();
        }
        if r2 < 2.0 {
            todo!();
        }

        let num_vertices = num_points * 2;
        let vertex_iter = (0..num_vertices).map(|i| {
            let angle =
                fj::Angle::from_rad(2. * PI / num_vertices as f64 * i as f64);
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

        let outer = fj::Sketch::from_points(outer);
        let inner = fj::Sketch::from_points(inner);

        let footprint =
            fj::Difference2d::from_shapes([outer.into(), inner.into()]);

        let star = fj::Sweep::from_path(footprint.into(), [0., 0., h]);

        Ok(star.into())
    }

    fn metadata(&self) -> ModelMetadata {
        ModelMetadata::new("Star")
            .with_argument(
                ArgumentMetadata::new("num_points").with_default_value("5"),
            )
            .with_argument(
                ArgumentMetadata::new("r1").with_default_value("1.0"),
            )
            .with_argument(
                ArgumentMetadata::new("r2").with_default_value("2.0"),
            )
            .with_argument(ArgumentMetadata::new("h").with_default_value("1.0"))
    }
}
