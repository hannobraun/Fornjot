use crate::Model;

/// An abstract interface to the Fornjot host.
pub trait Host {
    /// Register a model.
    ///
    /// This is mainly for more advanced use cases (e.g. when you need to close
    /// over extra state to load the model). For simpler models, you probably
    /// want to use [`HostExt::register_model()`] instead.
    #[doc(hidden)]
    fn register_boxed_model(&mut self, model: Box<dyn Model>);
}

impl<H: Host + ?Sized> Host for &'_ mut H {
    fn register_boxed_model(&mut self, model: Box<dyn Model>) {
        (*self).register_boxed_model(model);
    }
}

impl<H: Host + ?Sized> Host for Box<H> {
    fn register_boxed_model(&mut self, model: Box<dyn Model>) {
        (**self).register_boxed_model(model);
    }
}

/// Extension methods to augment the [`Host`] API.
pub trait HostExt {
    /// Register a model with the Fornjot runtime.
    fn register_model<M>(&mut self, model: M)
    where
        M: Model + 'static;
}

impl<H: Host + ?Sized> HostExt for H {
    fn register_model<M>(&mut self, model: M)
    where
        M: Model + 'static,
    {
        self.register_boxed_model(Box::new(model));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_is_object_safe() {
        let _: &dyn Host;
    }
}
