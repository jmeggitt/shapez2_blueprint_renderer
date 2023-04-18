use obj::Obj;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::exit;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ModelTweaksConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    rotate_all: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scale_all: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset_all: Option<[f64; 3]>,
    models: HashMap<String, ModelTweaks>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ModelTweaks {
    #[serde(rename = "ignore")]
    Ignored,
    General {
        /// The path of this model
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<PathBuf>,
        /// Defer to another model's configuration
        #[serde(skip_serializing_if = "Option::is_none")]
        using: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        rotation: Option<[f64; 3]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        scale: Option<[f64; 3]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        offset: Option<[f64; 3]>,
    },
}

pub struct ModelLoader {
    _config: ModelTweaksConfig,
    resolved_models: HashMap<String, Model>,
    model_dir: PathBuf,
}

pub enum Model {
    Resolved {
        model: Obj,
        // transform: Trans
    },
    Missing,
}

impl ModelLoader {
    pub fn from_config_or_default<P: AsRef<Path>>(
        config_path: Option<P>,
        model_dir: PathBuf,
    ) -> Self {
        if let Some(path) = config_path {
            let file = match File::open(path) {
                Ok(file) => BufReader::new(file),
                Err(e) => {
                    eprintln!("Error: unable to read model tweaks file: {}", e);
                    exit(1);
                }
            };

            match serde_json::from_reader(file) {
                Ok(config) => {
                    return ModelLoader {
                        _config: config,
                        resolved_models: HashMap::new(),
                        model_dir,
                    }
                }
                Err(e) => {
                    eprintln!("Error: failed to read model tweaks file: {}", e);
                    exit(1);
                }
            }
        }

        ModelLoader {
            _config: ModelTweaksConfig::default(),
            resolved_models: HashMap::new(),
            model_dir,
        }
    }

    fn try_load_model<P: AsRef<Path>>(path: P) -> Option<Model> {
        if !path.as_ref().is_file() {
            return None;
        }

        let mut obj = match Obj::load(path.as_ref()) {
            Ok(obj) => obj,
            Err(e) => {
                eprintln!(
                    "Error: Failed to read model {}: {}",
                    path.as_ref().display(),
                    e
                );
                return Some(Model::Missing);
            }
        };

        if let Err(e) = obj.load_mtls() {
            eprintln!("Error: failed to load model materials: {}", e);
            return Some(Model::Missing);
        }

        println!("Loaded model for {}", path.as_ref().display());
        Some(Model::Resolved { model: obj })
    }

    pub fn load_model(&mut self, name: &str) -> &Model {
        self.ensure_model_loaded(name);
        self.resolved_models.get(name).unwrap()
    }

    fn ensure_model_loaded(&mut self, name: &str) {
        if self.resolved_models.contains_key(name) {
            return;
        }

        // TODO: Use the tweaks instead of just hard coding everything
        let mut file = internal_name_mapping_adjustments(name);

        let path = self.model_dir.join(format!("{}.obj", file));
        if let Some(model) = Self::try_load_model(path) {
            self.resolved_models.insert(name.to_owned(), model);
            return;
        }

        if let Some(remaining) = file.strip_suffix("InternalVariant") {
            file = remaining;

            let path = self.model_dir.join(format!("{}.obj", file));
            if let Some(model) = Self::try_load_model(path) {
                self.resolved_models.insert(name.to_owned(), model);
                return;
            }
        }

        if let Some(remaining) = file.strip_suffix("Default") {
            file = remaining;
            let path = self.model_dir.join(format!("{}.obj", file));
            if let Some(model) = Self::try_load_model(path) {
                self.resolved_models.insert(name.to_owned(), model);
                return;
            }
        }

        println!(
            "Failed to find model for {}; building will not be rendered",
            name
        );
        self.resolved_models.insert(name.to_owned(), Model::Missing);
    }

    pub fn load_counts(&self) -> (usize, usize) {
        let resolved = self
            .resolved_models
            .iter()
            .filter(|(_, x)| matches!(x, Model::Resolved { .. }))
            .count();
        (resolved, self.resolved_models.len())
    }
}

fn internal_name_mapping_adjustments(internal_name: &str) -> &str {
    match internal_name {
        //belts
        "BeltDefaultForwardInternalVariant" => "Belt_Straight",
        "BeltDefaultRightInternalVariant" => "Belt_90_R",
        "BeltDefaultLeftInternalVariant" => "Belt_90_L",
        //vertical
        "Lift1UpBackwardInternalVariant" => "Lift1UpBackwards",
        //belts special
        "SplitterTShapeInternalVariant" => "Splitter2to1T",
        "MergerTShapeInternalVariant" => "Merger2to1T",
        "BeltPortSenderInternalVariant" => "BeltPortSender",
        "BeltPortReceiverInternalVariant" => "BeltPortReceiver",

        //rotating
        "RotatorOneQuadInternalVariant" => "Rotator1QuadPlatform90CC", // arrows onlu
        "RotatorOneQuadCCWInternalVariant" => "Rotator1QuadPlatform90CW", // ^
        "RotatorHalfInternalVariant" => "Rotator1QuadPlatform180", // ^^

        //processing
        "CutterDefaultInternalVariant" => "CutterStatic_Fixed",
        "StackerDefaultInternalVariant" => "StackerSolid",
        "PainterDefaultInternalVariant" => "PainterBasin",
        "MixerDefaultInternalVariant" => "MixerFoundation",
        "CutterHalfInternalVariant" => "HalfCutter",

        //pipes normal
        "PipeLeftInternalVariant" => "PipeLeftGlas",
        "PipeRightInternalVariant" => "PipeRightGlas",
        "PipeCrossInternalVariant" => "PipeCrossJunctionGlas",
        "PipeJunctionInternalVariant" => "PipeJunctionGlas",
        //pipes up
        "PipeUpForwardInternalVariant" => "Pipe1UpForwardGlas",
        "PipeUpBackwardInternalVariant" => "Pipe1UpBackwardGlas",
        "PipeUpLeftInternalVariant" => "Pipe1UpLeftBlueprint", // Contains the pump
        "PipeUpRightInternalVariant" => "Pipe1UpRightBlueprint", // ^
        //pipes down
        "PipeDownForwardInternalVariant" => "Pipe1DownGlas", 
        "PipeDownBackwardInternalVariant" => "Pipe1DownBackwardGlas",
        "PipeDownRightInternalVariant" => "Pipe1DownRightGlas",
        "PipeDownLeftInternalVariant" => "Pipe1DownLeftGlas",

        // Support Buildings
        "LabelDefaultInternalVariant" => "LabelSupport",
        "FluidStorageDefaultInternalVariant" => "PaintTankFoundation",
        "StorageDefaultInternalVariant" => "StorageSolid",
        "SandboxFluidProducerDefaultInternalVariant" => "SandboxIFluidProducer",
        x => x,
    }
}
