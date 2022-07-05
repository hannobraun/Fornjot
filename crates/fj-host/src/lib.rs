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
    path::PathBuf,
    process::Command,
    sync::mpsc,
    thread,
};

use notify::Watcher as _;
use thiserror::Error;

use self::platform::HostPlatform;

use fj_proto_param_cfg::ParamConfig;

/// Represents a Fornjot model
pub struct Model {
    src_path: PathBuf,
    lib_path: PathBuf,
    manifest_path: PathBuf,
    parameters_config: Vec<ParamConfig>,
}

impl Model {
    /// Initialize the model from a path
    ///
    /// Optionally, the target directory where plugin files are compiled to can
    /// be provided. If it is not provided, the target directory is assumed to
    /// be located within the model path.
    pub fn from_path(
        path: PathBuf,
        target_dir: Option<PathBuf>,
    ) -> io::Result<Self> {
        let name = {
            // Can't panic. It only would, if the path ends with "..", and we
            // are canonicalizing it here to prevent that.
            let canonical = path.canonicalize()?;
            let file_name = canonical
                .file_name()
                .expect("Expected path to be canonical");

            file_name.to_string_lossy().replace('-', "_")
        };

        let src_path = path.join("src");

        let lib_path = {
            let file = HostPlatform::lib_file_name(&name);
            let target_dir = target_dir.unwrap_or_else(|| path.join("target"));
            target_dir.join("debug").join(file)
        };

        let manifest_path = path.join("Cargo.toml");

        let model_src_path = src_path.join("lib.rs"); // TODO: Obtain this properly.

        dbg!(&model_src_path);

        let parameters_config = fj_proto_param_cfg::from_file_path(
            &model_src_path
                .to_str()
                .expect("Failed to get model source file path."),
        )
        .ok()
        .unwrap_or_default();

        dbg!(&parameters_config);

        Ok(Self {
            src_path,
            lib_path,
            manifest_path,
            parameters_config,
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
            let model: libloading::Symbol<ModelFn> = lib.get(b"model")?;
            model(arguments)
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

/// Parameters that are passed to a model
pub struct Parameters(pub HashMap<String, String>);

impl Parameters {
    /// Construct an empty instance of `Parameters`
    pub fn empty() -> Self {
        Self(HashMap::new())
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

    /// Error while watching the model code for changes
    #[error("Error watching model for changes")]
    Notify(#[from] notify::Error),
}

type ModelFn = unsafe extern "C" fn(args: &Parameters) -> fj::Shape;
