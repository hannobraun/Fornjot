use std::{marker::PhantomData, os::raw::c_void, panic::AssertUnwindSafe};

use crate::abi::Model;

/// A FFI-safe `&mut dyn Host`.
#[repr(C)]
pub struct Host<'a> {
    user_data: *mut c_void,
    register_boxed_model: unsafe extern "C" fn(*mut c_void, model: Model),
    _lifetime: PhantomData<&'a mut ()>,
}

impl<'a, H: crate::Host + Sized> From<&'a mut H> for Host<'a> {
    fn from(host: &'a mut H) -> Self {
        extern "C" fn register_boxed_model<H: crate::Host + Sized>(
            user_data: *mut c_void,
            model: Model,
        ) {
            let host = unsafe { &mut *(user_data as *mut H) };

            if let Err(e) = std::panic::catch_unwind(AssertUnwindSafe(|| {
                host.register_boxed_model(Box::new(model))
            })) {
                crate::abi::on_panic(e);
            }
        }

        Host {
            user_data: host as *mut H as *mut c_void,
            register_boxed_model: register_boxed_model::<H>,
            _lifetime: PhantomData,
        }
    }
}

impl<'a> crate::Host for Host<'a> {
    fn register_boxed_model(&mut self, model: Box<dyn crate::Model>) {
        let Host {
            user_data,
            register_boxed_model,
            ..
        } = *self;

        unsafe {
            register_boxed_model(user_data, model.into());
        }
    }
}
