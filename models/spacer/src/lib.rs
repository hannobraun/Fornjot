use fj::{
    syntax::*, ArgumentMetadata, ContextExt, HostExt, Metadata, Model,
    ModelMetadata,
};

fj::register_model!(|host| {
    host.register_model(Spacer);

    Ok(Metadata::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    ))
});

struct Spacer;

impl Model for Spacer {
    fn shape(
        &self,
        ctx: &dyn fj::Context,
    ) -> Result<fj::Shape, Box<dyn std::error::Error + Send + Sync>> {
        let outer: f64 = ctx.parse_optional_argument("outer")?.unwrap_or(1.0);
        let inner: f64 = ctx.parse_optional_argument("inner")?.unwrap_or(0.5);
        let height: f64 = ctx.parse_optional_argument("height")?.unwrap_or(1.0);

        if outer < inner * 1.01 {
            todo!("Return a suitable error");
        }
        if inner > outer * 0.99 {
            todo!("Return a suitable error");
        }

        let outer_edge =
            fj::Sketch::from_circle(fj::Circle::from_radius(outer));
        let inner_edge =
            fj::Sketch::from_circle(fj::Circle::from_radius(inner));

        let footprint = outer_edge.difference(&inner_edge);
        let spacer = footprint.sweep([0., 0., height]);

        Ok(spacer.into())
    }

    fn metadata(&self) -> ModelMetadata {
        ModelMetadata::new("Spacer")
            .with_argument(
                ArgumentMetadata::new("outer").with_default_value("1.0"),
            )
            .with_argument(
                ArgumentMetadata::new("inner").with_default_value("0.5"),
            )
            .with_argument(
                ArgumentMetadata::new("height").with_default_value("1.0"),
            )
    }
}
