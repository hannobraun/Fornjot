use std::{marker::PhantomData, os::raw::c_void, panic::AssertUnwindSafe};

use crate::abi::wrappers::StringSlice;

#[repr(C)]
pub struct Context<'a> {
    user_data: *const c_void,
    get_argument:
        unsafe extern "C" fn(*const c_void, StringSlice) -> StringSlice,
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> From<&'a &dyn crate::Context> for Context<'a> {
    fn from(ctx: &'a &dyn crate::Context) -> Self {
        unsafe extern "C" fn get_argument(
            user_data: *const c_void,
            name: StringSlice,
        ) -> StringSlice {
            let ctx = &*(user_data as *const &dyn crate::Context);

            match std::panic::catch_unwind(AssertUnwindSafe(|| {
                ctx.get_argument(&*name)
            })) {
                Ok(Some(arg)) => StringSlice::from_str(arg),
                Ok(None) => StringSlice::from_str(""),
                Err(payload) => crate::abi::on_panic(payload),
            }
        }

        Context {
            user_data: ctx as *const &dyn crate::Context as *const c_void,
            get_argument,
            _lifetime: PhantomData,
        }
    }
}

impl crate::Context for Context<'_> {
    fn get_argument(&self, name: &str) -> Option<&str> {
        unsafe {
            let Context {
                user_data,
                get_argument,
                _lifetime,
            } = *self;

            let name = StringSlice::from_str(name);

            match get_argument(user_data, name).into_str() {
                "" => None,
                other => Some(other),
            }
        }
    }
}
