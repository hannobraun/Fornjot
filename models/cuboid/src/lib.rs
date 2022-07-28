use fj::{
    ArgumentMetadata, ContextExt, HostExt, Model, ModelMetadata, PluginMetadata,
};

fj::register_model!(|host| {
    host.register_model(Cuboid);

    Ok(PluginMetadata::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    ))
});

pub struct Cuboid;

impl Model for Cuboid {
    fn shape(
        &self,
        ctx: &dyn fj::Context,
    ) -> Result<fj::Shape, Box<dyn std::error::Error + Send + Sync>> {
        let x: f64 = ctx.parse_optional_argument("x")?.unwrap_or(3.0);
        let y: f64 = ctx.parse_optional_argument("y")?.unwrap_or(2.0);
        let z: f64 = ctx.parse_optional_argument("z")?.unwrap_or(1.0);

        #[rustfmt::skip]
        let rectangle = fj::Sketch::from_points(vec![
            [-x / 2., -y / 2.],
            [ x / 2., -y / 2.],
            [ x / 2.,  y / 2.],
            [-x / 2.,  y / 2.],
        ]).with_color([100,255,0,200]);

        let cuboid = fj::Sweep::from_path(rectangle.into(), [0., 0., z]);

        Ok(cuboid.into())
    }

    fn metadata(&self) -> ModelMetadata {
        ModelMetadata::new("Cuboid")
            .with_argument(ArgumentMetadata::new("x").with_default_value("3.0"))
            .with_argument(ArgumentMetadata::new("y").with_default_value("2.0"))
            .with_argument(ArgumentMetadata::new("z").with_default_value("1.0"))
    }
}
