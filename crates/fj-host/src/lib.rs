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
    collections::{BTreeMap, HashMap, HashSet},
    ffi::OsStr,
    io,
    path::PathBuf,
    process::Command,
    str::FromStr,
    sync::mpsc::{self, SyncSender},
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
    tx3: Option<SyncSender<()>>, // Used to implement "refresh on parameter value change".
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
            tx3: None,
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
        mut self,
        mut parameters: Parameters,
    ) -> Result<Watcher, Error> {
        let (tx, rx) = mpsc::sync_channel(0);
        let tx2 = tx.clone();

        self.tx3 = Some(tx.clone()); // Used to implement "refresh on parameter value change".

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

        parameters.init_from_config(&self.parameters_config);

        Ok(Watcher {
            _watcher: Box::new(watcher),
            channel: rx,
            model: self,
            parameters,
        })
    }

    ///
    /// This function requests that the model be
    /// regenerated due to a change of parameter values.
    ///
    /// Ideally the refresh will occur without recompilation
    /// of the model.
    ///
    pub fn refresh(&self) {
        //
        // Hacky implemention of "refresh on parameter value change"
        // feature.
        //
        // This piggy-backs on top of the existing "model source change"
        // notification implementation which seemed like the most
        // obvious implementation approach.
        //
        // Ideally this could be incorporated into the current
        // Watcher implementation more transparently.
        //
        // Also, this currently results in `cargo build` being
        // executed unnecessarily when the model regeneration
        // could just re-use the existing binary for parameter
        // value changes.
        //
        // NOTE: Calls to this function are *not* currently
        //       throttled in any way at this level of the
        //       system[0].
        //
        //       We should probably only allow one pending
        //       reload request at a time.
        //
        //       Excessive calls to this function (e.g. multiple
        //       times per frame) have lead to Bad Things(TM)
        //       such as Window Manager segfaults during
        //       development.
        //
        //       [0] The `reload_requested` related code in `run.rs`
        //           does attempt to limit to one call per frame.
        //
        if let Some(tx3) = &self.tx3 {
            let tx = tx3.clone();
            thread::spawn(move || {
                tx.send(()).expect("Channel is disconnected")
            });
        }
    }
}

/// Watches a model for changes, reloading it continually
pub struct Watcher {
    _watcher: Box<dyn notify::Watcher>,
    channel: mpsc::Receiver<()>,
    model: Model,
    ///
    /// `parameters` is "temporarily" public so that it can be
    /// accessed by UI-related code.
    ///
    pub parameters: Parameters,
}

impl Watcher {
    ///
    /// Request that the model is regenerated due to parameter value
    /// change(s), e.g.via the UI.
    ///
    /// Handles synchronization of the UI value representation (f64)
    /// with the model plugin value (String) representation.
    ///
    /// The updated shape will appear via `receive()` as usual.
    ///
    pub fn refresh(&mut self) {
        self.parameters.update_string_values_hack();
        self.model.refresh();
    }

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
#[derive(Default)]
pub struct Parameters(
    pub HashMap<String, String>,
    BTreeMap<String, ParamConfig>,
    pub BTreeMap<String, f64>, // Part of workaround for parameter value String vs f64 issue.
);

impl Parameters {
    /// Construct an empty instance of `Parameters`
    pub fn empty() -> Self {
        Self(HashMap::new(), Default::default(), Default::default())
    }

    ///
    /// "Initialize" the parameters based on the defaults supplied
    /// by the model source file.
    ///
    /// The default values will be used _unless_ the parameter value
    /// already exists, e.g. due to being supplied on command line.
    ///
    /// No validation of the previously supplied values is performed.
    ///
    //
    //  This function also stores the provided parameter configuration
    //  for further use, e.g. display in the UI.
    //
    //  It probably makes more sense to create the instance based on
    //  the parameter configuration and _then_ update it with the values
    //  supplied on the command line. But for the initial proof-of-concept
    //  current approach required fewer changes to other parts of the code.
    //
    pub fn init_from_config(&mut self, parameters_config: &[ParamConfig]) {
        let current_params = &mut self.0; // Tuple struct backwards compatibility hack.
        let configs = &mut self.1; // Tuple struct backwards compatibility hack.
        let f64_values = &mut self.2; // Tuple struct backwards compatibility hack.

        for cfg in parameters_config {
            let v = current_params.entry(cfg.name()).or_insert(cfg.default());
            configs.insert(cfg.name(), cfg.clone()); // Note: Not very robust.

            // Ensures the f64 values used for GUI display are synchronized
            // with the initial string values passed to the model.
            *f64_values.entry(cfg.name()).or_default() =
                f64::from_str(v).unwrap_or_default();
        }
    }

    fn update_string_values_hack(&mut self) {
        //
        // This function is part of a workaround/hack to deal with the issue
        // that we pass parameter values to models as strings but we want
        // to use numeric values in the UI.
        //
        // Additionally, we currently give `egui` direct access to the f64
        // values for modification by UI elements.
        //
        // This function needs to be called after the f64 values are
        // modified via the GUI so that the string values passed to the
        // model are synchronized with the modified f64 values.
        //
        // TODO: Figure out a better way of handling this?
        //
        let current_string_params = &mut self.0; // Tuple struct backwards compatibility hack.
        let current_f64_values = &self.2; // Tuple struct backwards compatibility hack.

        for (k, v) in current_string_params {
            if let Some(value_as_f64) = current_f64_values.get(k) {
                *v = value_as_f64.to_string();
            }
        }
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
