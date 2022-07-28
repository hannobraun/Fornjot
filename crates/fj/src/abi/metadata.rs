use crate::abi::FfiSafeString;

#[derive(Debug)]
#[repr(C)]
pub struct ModelMetadata {
    pub name: FfiSafeString,
}

impl From<ModelMetadata> for crate::ModelMetadata {
    fn from(_m: ModelMetadata) -> Self {
        todo!()
    }
}

impl From<crate::ModelMetadata> for ModelMetadata {
    fn from(_m: crate::ModelMetadata) -> Self {
        todo!()
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct PluginMetadata {}

impl From<PluginMetadata> for crate::PluginMetadata {
    fn from(_m: PluginMetadata) -> Self {
        todo!()
    }
}

impl From<crate::PluginMetadata> for PluginMetadata {
    fn from(_m: crate::PluginMetadata) -> Self {
        todo!()
    }
}
