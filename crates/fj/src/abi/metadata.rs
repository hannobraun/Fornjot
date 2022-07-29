use crate::abi::ffi_safe;

#[derive(Debug)]
#[repr(C)]
pub struct ModelMetadata {
    name: ffi_safe::String,
    description: ffi_safe::Option<ffi_safe::String>,
    arguments: ffi_safe::Vec<ArgumentMetadata>,
}

impl From<ModelMetadata> for crate::ModelMetadata {
    fn from(m: ModelMetadata) -> Self {
        let ModelMetadata {
            name,
            description,
            arguments,
        } = m;

        crate::ModelMetadata {
            name: name.into(),
            description: description.map(Into::into).into(),
            arguments: arguments.iter().cloned().map(|a| a.into()).collect(),
        }
    }
}

impl From<crate::ModelMetadata> for ModelMetadata {
    fn from(_m: crate::ModelMetadata) -> Self {
        todo!()
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Metadata {
    name: ffi_safe::String,
    version: ffi_safe::String,
    short_description: ffi_safe::Option<ffi_safe::String>,
    description: ffi_safe::Option<ffi_safe::String>,
    homepage: ffi_safe::Option<ffi_safe::String>,
    repository: ffi_safe::Option<ffi_safe::String>,
    license: ffi_safe::Option<ffi_safe::String>,
}

impl From<Metadata> for crate::Metadata {
    fn from(m: Metadata) -> Self {
        let Metadata {
            name,
            version,
            short_description,
            description,
            homepage,
            repository,
            license,
        } = m;

        crate::Metadata {
            name: name.into(),
            version: version.into(),
            short_description: short_description.map(Into::into).into(),
            description: description.map(Into::into).into(),
            homepage: homepage.map(Into::into).into(),
            repository: repository.map(Into::into).into(),
            license: license.map(Into::into).into(),
        }
    }
}

impl From<crate::Metadata> for Metadata {
    fn from(m: crate::Metadata) -> Self {
        let crate::Metadata {
            name,
            version,
            short_description,
            description,
            homepage,
            repository,
            license,
        } = m;

        Metadata {
            name: name.into(),
            version: version.into(),
            short_description: short_description.into(),
            description: description.into(),
            homepage: homepage.into(),
            repository: repository.into(),
            license: license.into(),
        }
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct ArgumentMetadata {
    name: ffi_safe::String,
    description: ffi_safe::Option<ffi_safe::String>,
    default_value: ffi_safe::Option<ffi_safe::String>,
}

impl From<crate::ArgumentMetadata> for ArgumentMetadata {
    fn from(meta: crate::ArgumentMetadata) -> Self {
        let crate::ArgumentMetadata {
            name,
            description,
            default_value,
        } = meta;

        ArgumentMetadata {
            name: name.into(),
            description: description.into(),
            default_value: default_value.into(),
        }
    }
}

impl From<ArgumentMetadata> for crate::ArgumentMetadata {
    fn from(meta: ArgumentMetadata) -> Self {
        let ArgumentMetadata {
            name,
            description,
            default_value,
        } = meta;

        crate::ArgumentMetadata {
            name: name.into(),
            description: description.map(Into::into).into(),
            default_value: default_value.map(Into::into).into(),
        }
    }
}
