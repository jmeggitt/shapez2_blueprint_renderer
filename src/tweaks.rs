use nalgebra_glm::Vec3;
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
    resolved_models: HashMap<String, Vec<Model>>,
    model_dir: PathBuf,
}

pub enum Model {
    Resolved {
        model: Obj,
        offset: Vec3
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

    fn try_load_model<P: AsRef<Path>>(path: P, offset: Vec3) -> Option<Model> {
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
        Some(Model::Resolved { model: obj, offset })
    }

    pub fn load_model(&mut self, name: &str) -> &Vec<Model> {
        self.ensure_model_loaded(name);
        self.resolved_models.get(name).unwrap()
    }

    fn ensure_model_loaded(&mut self, name: &str) {
        if self.resolved_models.contains_key(name) {
            return;
        }

        // TODO: Use the tweaks instead of just hard coding everything
        let files = internal_name_mapping_adjustments(name);
        let mut models = vec![];
        
        for mut entry in files {
            let path = self.model_dir.join(format!("{}.obj", entry.file));
            if let Some(model) = Self::try_load_model(path, entry.offset) {
                models.push(model);
                continue;
            }

            if let Some(remaining) = entry.file.strip_suffix("InternalVariant") {
                entry.file = remaining.to_owned();

                let path = self.model_dir.join(format!("{}.obj", entry.file));
                if let Some(model) = Self::try_load_model(path, entry.offset) {
                    models.push(model);
                    continue;
                }
            }

            if let Some(remaining) = entry.file.strip_suffix("Default") {
               entry.file = remaining.to_owned();
                let path = self.model_dir.join(format!("{}.obj", entry.file));
                if let Some(model) = Self::try_load_model(path, entry.offset) {
                    models.push(model);
                    continue;
                }
            }
        } 
        
        if !models.is_empty() {
            self.resolved_models.insert(name.to_owned(), models);
            return;
        }

        println!(
            "Failed to find model for {}; building will not be rendered",
            name
        );
        self.resolved_models.insert(name.to_owned(), vec![]);
    }

    pub fn load_counts(&self) -> (usize, usize) {
        let resolved = self
            .resolved_models
            .iter()
            .filter(|(_, x)| x.iter().any(|y| matches!(y, Model::Resolved { .. })))
            .count();
        (resolved, self.resolved_models.len())
    }
}

#[derive(Clone)]
pub struct Mapping {
    file: String,
    offset: Vec3
}

fn internal_name_mapping_adjustments(internal_name: &str) -> Vec<Mapping> {
    match internal_name {
        //belts
        "BeltDefaultForwardInternalVariant" => vec![Mapping {
            file: "Belt_Straight".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "BeltDefaultRightInternalVariant" => vec![Mapping {
            file: "Belt_90_R".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "BeltDefaultLeftInternalVariant" => vec![Mapping {
            file: "Belt_90_L".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        //vertical
        "Lift1UpBackwardInternalVariant" => vec![Mapping {
            file: "Lift1UpBackwards".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        //belts special
        "SplitterTShapeInternalVariant" => vec![Mapping {
            file: "Splitter2to1T".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "MergerTShapeInternalVariant" => vec![Mapping {
            file: "Merger2to1T".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "BeltPortSenderInternalVariant" => vec![Mapping {
            file: "BeltPortSender".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "BeltPortReceiverInternalVariant" => vec![Mapping {
            file: "BeltPortReceiver".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],

        //rotating
        "RotatorOneQuadInternalVariant" => vec![Mapping {
            file: "Rotator1Quad".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }, Mapping {
            file: "Rotator1QuadPlatform90CC".to_string(),
            offset: Vec3::new(0.0, 0.05, 0.0)
        }], // arrows onlu
        "RotatorOneQuadCCWInternalVariant" => vec![Mapping {
            file: "Rotator1Quad".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }, Mapping {
            file: "Rotator1QuadPlatform90CW".to_string(),
            offset: Vec3::new(0.0, 0.05, 0.0)
        }], // ^
        "RotatorHalfInternalVariant" => vec![Mapping {
            file: "Rotator1Quad".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }, Mapping {
            file: "Rotator1QuadPlatform180".to_string(),
            offset: Vec3::new(0.0, 0.05, 0.0)
        }], // ^^

        //processing
        "CutterDefaultInternalVariant" => vec![Mapping {
            file: "CutterStatic_Fixed".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "StackerDefaultInternalVariant" => vec![Mapping {
            file: "StackerSolid".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "PainterDefaultInternalVariant" => vec![Mapping {
            file: "PainterBasin".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "MixerDefaultInternalVariant" => vec![Mapping {
            file: "MixerFoundation".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "CutterHalfInternalVariant" => vec![Mapping {
            file: "HalfCutter".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],

        //pipes normal
        "PipeLeftInternalVariant" => vec![Mapping {
            file: "PipeLeftGlas".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "PipeRightInternalVariant" => vec![Mapping {
            file: "PipeRightGlas".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "PipeCrossInternalVariant" => vec![Mapping {
            file: "PipeCrossJunctionGlas".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "PipeJunctionInternalVariant" => vec![Mapping {
            file: "PipeJunctionGlas".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        //pipes up
        "PipeUpForwardInternalVariant" => vec![Mapping {
            file: "Pipe1UpForwardGlas".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "PipeUpBackwardInternalVariant" => vec![Mapping {
            file: "Pipe1UpBackwardGlas".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "PipeUpLeftInternalVariant" => vec![Mapping {
            file: "Pipe1UpLeftBlueprint".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }], // Contains the pump
        "PipeUpRightInternalVariant" => vec![Mapping {
            file: "Pipe1UpRightBlueprint".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }], // ^
        //pipes down
        "PipeDownForwardInternalVariant" => vec![Mapping {
            file: "Pipe1DownGlas".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }], 
        "PipeDownBackwardInternalVariant" => vec![Mapping {
            file: "Pipe1DownBackwardGlas".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "PipeDownRightInternalVariant" => vec![Mapping {
            file: "Pipe1DownRightGlas".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "PipeDownLeftInternalVariant" => vec![Mapping {
            file: "Pipe1DownLeftGlas".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],

        // Support Buildings
        "LabelDefaultInternalVariant" => vec![Mapping {
            file: "LabelSupport".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "FluidStorageDefaultInternalVariant" => vec![Mapping {
            file: "PaintTankFoundation".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "StorageDefaultInternalVariant" => vec![Mapping {
            file: "StorageSolid".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        "SandboxFluidProducerDefaultInternalVariant" => vec![Mapping {
            file: "SandboxIFluidProducer".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
        x => vec![Mapping {
            file: x.to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0)
        }],
    }
}
