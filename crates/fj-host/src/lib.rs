//! # Fornjot Model Host
//!
//! This library is part of the [Fornjot] ecosystem. Fornjot is an open-source,
//! code-first CAD application; and collection of libraries that make up the CAD
//! application, but can be used independently.
//!
//! This library is an internal component of Fornjot. It is not relevant to end
//! users that just want to create CAD models.
//!
//! The purpose of this library is to load Fornjot models and watch them for
//! changes. Fornjot models are basically plugins that can be loaded into a CAD
//! application. This library is the host for these model plugins.
//!
//! [Fornjot]: https://www.fornjot.app/

#![warn(missing_docs)]

mod platform;

use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
    thread,
};

use fj::abi;
use notify::Watcher as _;
use thiserror::Error;

use self::platform::HostPlatform;

/// Represents a Fornjot model
pub struct Model {
    src_path: PathBuf,
    lib_path: PathBuf,
    manifest_path: PathBuf,
}

impl Model {
    /// Initialize the model using the path to its crate (i.e. the folder
    /// containing `Cargo.toml`).
    ///
    /// Optionally, the target directory where plugin files are compiled to can
    /// be provided. If it is not provided, the target directory is assumed to
    /// be located within the model path.
    pub fn from_path(path: PathBuf) -> Result<Self, Error> {
        let crate_dir = path.canonicalize()?;

        let metadata = cargo_metadata::MetadataCommand::new()
            .current_dir(&crate_dir)
            .exec()?;

        let pkg = package_associated_with_directory(&metadata, &crate_dir)?;
        let src_path = crate_dir.join("src");

        let lib_path = {
            let name = pkg.name.replace('-', "_");
            let file = HostPlatform::lib_file_name(&name);
            let target_dir =
                metadata.target_directory.clone().into_std_path_buf();
            target_dir.join("debug").join(file)
        };

        Ok(Self {
            src_path,
            lib_path,
            manifest_path: pkg.manifest_path.as_std_path().to_path_buf(),
        })
    }

    /// Load the model once
    ///
    /// The passed arguments are provided to the model. Returns the shape that
    /// the model returns.
    ///
    /// Please refer to [`Model::load_and_watch`], if you want to watch the
    /// model for changes, reloading it continually.
    pub fn load_once(
        &self,
        arguments: &Parameters,
    ) -> Result<fj::Shape, Error> {
        let manifest_path = self.manifest_path.display().to_string();

        let status = Command::new("cargo")
            .arg("build")
            .args(["--manifest-path", &manifest_path])
            .status()?;

        if !status.success() {
            return Err(Error::Compile);
        }

        // So, strictly speaking this is all unsound:
        // - `Library::new` requires us to abide by the arbitrary requirements
        //   of any library initialization or termination routines.
        // - `Library::get` requires us to specify the correct type for the
        //   model function.
        // - The model function itself is `unsafe`, because it is a function
        //   from across an FFI interface.
        //
        // Typical models won't have initialization or termination routines (I
        // think), should abide by the `ModelFn` signature, and might not do
        // anything unsafe. But we have no way to know that the library the user
        // told us to load actually does (I think).
        //
        // I don't know of a way to fix this. We should take this as motivation
        // to switch to a better technique:
        // https://github.com/hannobraun/Fornjot/issues/71
        let shape = unsafe {
            let lib = libloading::Library::new(&self.lib_path)?;
            let init: libloading::Symbol<abi::InitFunction> =
                lib.get(abi::INIT_FUNCTION_NAME.as_bytes())?;

            let mut host = Host {
                args: arguments,
                model: None,
            };

            match init(&mut abi::Host::from(&mut host)) {
                abi::ffi_safe::Result::Ok(_metadata) => {}
                abi::ffi_safe::Result::Err(e) => {
                    return Err(Error::InitializeModel(e.into()));
                }
            }

            let model = host.model.take().ok_or(Error::NoModelRegistered)?;

            model.shape(&host).map_err(Error::Shape)?
        };

        Ok(shape)
    }

    /// Load the model, then watch it for changes
    ///
    /// Whenever a change is detected, the model is being reloaded.
    ///
    /// Consumes this instance of `Model` and returns a [`Watcher`], which can
    /// be queried for changes to the model.
    pub fn load_and_watch(
        self,
        parameters: Parameters,
    ) -> Result<Watcher, Error> {
        let (tx, rx) = mpsc::sync_channel(0);
        let tx2 = tx.clone();

        let watch_path = self.src_path.clone();

        let mut watcher = notify::recommended_watcher(
            move |event: notify::Result<notify::Event>| {
                // Unfortunately the `notify` documentation doesn't say when
                // this might happen, so no idea if it needs to be handled.
                let event = event.expect("Error handling watch event");

                // Various acceptable ModifyKind kinds. Varies across platforms
                // (e.g. MacOs vs. Windows10)
                if let notify::EventKind::Modify(
                    notify::event::ModifyKind::Any,
                )
                | notify::EventKind::Modify(
                    notify::event::ModifyKind::Data(
                        notify::event::DataChange::Any,
                    ),
                )
                | notify::EventKind::Modify(
                    notify::event::ModifyKind::Data(
                        notify::event::DataChange::Content,
                    ),
                ) = event.kind
                {
                    let file_ext = event
                        .paths
                        .get(0)
                        .expect("File path missing in watch event")
                        .extension();

                    let black_list = HashSet::from([
                        OsStr::new("swp"),
                        OsStr::new("tmp"),
                        OsStr::new("swx"),
                    ]);

                    if let Some(ext) = file_ext {
                        if black_list.contains(ext) {
                            return;
                        }
                    }

                    // This will panic, if the other end is disconnected, which
                    // is probably the result of a panic on that thread, or the
                    // application is being shut down.
                    //
                    // Either way, not much we can do about it here.
                    tx.send(()).expect("Channel is disconnected");
                }
            },
        )?;

        watcher.watch(&watch_path, notify::RecursiveMode::Recursive)?;

        // To prevent a race condition between the initial load and the start of
        // watching, we'll trigger the initial load here, after having started
        // watching.
        //
        // Will panic, if the receiving end has panicked. Not much we can do
        // about that, if it happened.
        thread::spawn(move || tx2.send(()).expect("Channel is disconnected"));

        Ok(Watcher {
            _watcher: Box::new(watcher),
            channel: rx,
            model: self,
            parameters,
        })
    }
}

fn package_associated_with_directory<'m>(
    metadata: &'m cargo_metadata::Metadata,
    dir: &Path,
) -> Result<&'m cargo_metadata::Package, Error> {
    for pkg in metadata.workspace_packages() {
        let crate_dir = pkg
            .manifest_path
            .parent()
            .and_then(|p| p.canonicalize().ok());

        if crate_dir.as_deref() == Some(dir) {
            return Ok(pkg);
        }
    }

    Err(ambiguous_path_error(metadata, dir))
}

fn ambiguous_path_error(
    metadata: &cargo_metadata::Metadata,
    dir: &Path,
) -> Error {
    let mut possible_paths = Vec::new();

    for id in &metadata.workspace_members {
        let cargo_toml = &metadata[id].manifest_path;
        let crate_dir = cargo_toml
            .parent()
            .expect("A Cargo.toml always has a parent");
        // Try to make the path relative to the workspace root so error messages
        // aren't super long.
        let simplified_path = crate_dir
            .strip_prefix(&metadata.workspace_root)
            .unwrap_or(crate_dir);

        possible_paths.push(simplified_path.into());
    }

    Error::AmbiguousPath {
        dir: dir.to_path_buf(),
        possible_paths,
    }
}

/// Watches a model for changes, reloading it continually
pub struct Watcher {
    _watcher: Box<dyn notify::Watcher>,
    channel: mpsc::Receiver<()>,
    model: Model,
    parameters: Parameters,
}

impl Watcher {
    /// Receive an updated shape that the reloaded model created
    ///
    /// Returns `None`, if the model has not changed since the last time this
    /// method was called.
    pub fn receive(&self) -> Option<fj::Shape> {
        match self.channel.try_recv() {
            Ok(()) => {
                let shape = match self.model.load_once(&self.parameters) {
                    Ok(shape) => shape,
                    Err(Error::Compile) => {
                        // It would be better to display an error in the UI,
                        // where the user can actually see it. Issue:
                        // https://github.com/hannobraun/fornjot/issues/30
                        println!("Error compiling model");
                        return None;
                    }
                    Err(err) => {
                        panic!("Error reloading model: {:?}", err);
                    }
                };

                Some(shape)
            }
            Err(mpsc::TryRecvError::Empty) => {
                // Nothing to receive from the channel.
                None
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                // The other end has disconnected. This is probably the result
                // of a panic on the other thread, or a program shutdown in
                // progress. In any case, not much we can do here.
                panic!();
            }
        }
    }
}

/// Parameters that are passed to a model.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameters(pub HashMap<String, String>);

impl Parameters {
    /// Construct an empty instance of `Parameters`
    pub fn empty() -> Self {
        Self(HashMap::new())
    }

    /// Insert a value into the [`Parameters`] dictionary, implicitly converting
    /// the arguments to strings and returning `&mut self` to enable chaining.
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        value: impl ToString,
    ) -> &mut Self {
        self.0.insert(key.into(), value.to_string());
        self
    }
}

impl Deref for Parameters {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Parameters {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// An error that can occur when loading or reloading a model
#[derive(Debug, Error)]
pub enum Error {
    /// Model failed to compile
    #[error("Error compiling model")]
    Compile,

    /// I/O error while loading the model
    #[error("I/O error while loading model")]
    Io(#[from] io::Error),

    /// Failed to load the model's dynamic library
    #[error("Error loading model from dynamic library")]
    LibLoading(#[from] libloading::Error),

    /// Initializing a model failed.
    #[error("Unable to initialize the model")]
    InitializeModel(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// The user forgot to register a model when calling
    /// [`fj::register_model!()`].
    #[error("No model was registered")]
    NoModelRegistered,

    /// An error was returned from [`fj::Model::shape()`].
    #[error("Unable to determine the model's geometry")]
    Shape(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// Error while watching the model code for changes
    #[error("Error watching model for changes")]
    Notify(#[from] notify::Error),

    /// An error occurred while trying to use evaluate
    /// [`cargo_metadata::MetadataCommand`].
    #[error("Unable to determine the crate's metadata")]
    CargoMetadata(#[from] cargo_metadata::Error),

    /// The user pointed us to a directory, but it doesn't look like that was
    /// a crate root (i.e. the folder containing `Cargo.toml`).
    #[error(
        "It doesn't look like \"{}\" is a crate directory. Did you mean one of {}?",
        dir.display(),
        possible_paths.iter().map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
    )]
    AmbiguousPath {
        /// The model directory supplied by the user.
        dir: PathBuf,
        /// The directories for each crate in the workspace, relative to the
        /// workspace root.
        possible_paths: Vec<PathBuf>,
    },
}

struct Host<'a> {
    args: &'a Parameters,
    model: Option<Box<dyn fj::Model>>,
}

impl<'a> fj::Host for Host<'a> {
    fn register_boxed_model(&mut self, model: Box<dyn fj::Model>) {
        self.model = Some(model);
    }
}

impl<'a> fj::Context for Host<'a> {
    fn get_argument(&self, name: &str) -> Option<&str> {
        self.args.get(name).map(|s| s.as_str())
    }
}
