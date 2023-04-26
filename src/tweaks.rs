use log::{error, warn};
use nalgebra_glm::Vec3;
use obj::Obj;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::rc::Rc;

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
    resolved_objects: HashMap<String, Rc<Obj>>,
    model_sets: HashMap<String, Vec<Model>>,
    model_dir: PathBuf,
}

pub struct Model {
    pub model: Rc<Obj>,
    pub offset: Vec3,
}

impl ModelLoader {
    pub fn from_config_or_default<P: AsRef<Path>>(config_path: Option<P>, model_dir: P) -> Self {
        if let Some(path) = config_path {
            let file = match File::open(path) {
                Ok(file) => BufReader::new(file),
                Err(e) => {
                    error!("unable to read model tweaks file: {}", e);
                    exit(1);
                }
            };

            match serde_json::from_reader(file) {
                Ok(config) => {
                    return ModelLoader {
                        _config: config,
                        resolved_objects: HashMap::new(),
                        model_sets: HashMap::new(),
                        model_dir: model_dir.as_ref().to_path_buf(),
                    };
                }
                Err(e) => {
                    error!("failed to read model tweaks file: {}", e);
                    exit(1);
                }
            }
        }

        ModelLoader {
            _config: ModelTweaksConfig::default(),
            resolved_objects: HashMap::new(),
            model_sets: HashMap::new(),
            model_dir: model_dir.as_ref().to_path_buf(),
        }
    }

    fn try_load_object<P: AsRef<Path>>(path: P) -> Option<Obj> {
        if !path.as_ref().is_file() {
            return None;
        }

        let mut obj = match Obj::load(path.as_ref()) {
            Ok(obj) => obj,
            Err(e) => {
                error!("Failed to read model {}: {}", path.as_ref().display(), e);
                return None;
            }
        };

        if let Err(e) = obj.load_mtls() {
            error!("Error: failed to load model materials: {}", e);
            return None;
        }

        Some(obj)
    }

    fn find_object(&mut self, mut name: &str) -> Option<Rc<Obj>> {
        if let Some(obj) = self.resolved_objects.get(name) {
            return Some(obj.clone());
        }

        'search: loop {
            if let Some(obj) = Self::try_load_object(self.model_dir.join(format!("{}.obj", name))) {
                let reference_counted = Rc::new(obj);
                self.resolved_objects
                    .insert(name.to_owned(), reference_counted.clone());
                return Some(reference_counted);
            }

            for suffix in ["InternalVariant", "Default"] {
                if let Some(remaining) = name.strip_suffix(suffix) {
                    name = remaining;
                    continue 'search;
                }
            }

            return None;
        }
    }

    pub fn load_model(&mut self, name: &str) -> &[Model] {
        self.ensure_model_loaded(name);
        &self.model_sets.get(name).unwrap()[..]
    }

    fn ensure_model_loaded(&mut self, name: &str) {
        if self.model_sets.contains_key(name) {
            return;
        }

        let model_set = match internal_name_mapping_adjustments(name) {
            None => Vec::from_iter(self.find_object(name).map(|model| Model {
                model,
                offset: Vec3::default(),
            })),
            Some(mappings) => {
                Vec::from_iter(mappings.iter().filter_map(|&Mapping { file, offset }| {
                    Some(Model {
                        model: self.find_object(file)?,
                        offset,
                    })
                }))
            }
        };

        if model_set.is_empty() {
            warn!(
                "Failed to find model for {}; building will not be rendered",
                name
            );
        }

        self.model_sets.insert(name.to_owned(), model_set);
    }

    pub fn load_counts(&self) -> (usize, usize) {
        let resolved = self
            .model_sets
            .iter()
            .filter(|(_, x)| !x.is_empty())
            .count();
        (resolved, self.model_sets.len())
    }
}

#[derive(Copy, Clone)]
pub struct Mapping<'s> {
    file: &'s str,
    offset: Vec3,
}

impl<'s> Mapping<'s> {
    const fn new(file: &'s str, offset: Vec3) -> Self {
        Mapping { file, offset }
    }

    const fn redirect(file: &'s str) -> Self {
        Mapping {
            file,
            offset: Vec3::new(0.0, 0.0, 0.0),
        }
    }
}

macro_rules! const_mapping {
    ($($token:tt)*) => {{
        const X: &[Mapping] = &[$($token)*];
        X
    }};
}

fn internal_name_mapping_adjustments(internal_name: &str) -> Option<&'static [Mapping<'static>]> {
    Some(match internal_name {
        //belts
        "BeltDefaultForwardInternalVariant" => const_mapping![Mapping::redirect("Belt_Straight")],
        "BeltDefaultRightInternalVariant" => const_mapping![Mapping::redirect("Belt_90_R")],
        "BeltDefaultLeftInternalVariant" => const_mapping![Mapping::redirect("Belt_90_L")],
        //vertical
        "Lift1UpBackwardInternalVariant" => const_mapping![Mapping::redirect("Lift1UpBackwards")],
        //belts special
        "SplitterTShapeInternalVariant" => const_mapping![Mapping::redirect("Splitter2to1T")],
        "MergerTShapeInternalVariant" => const_mapping![Mapping::redirect("Merger2to1T")],
        "BeltPortSenderInternalVariant" => const_mapping![Mapping::redirect("BeltPortSender")],
        "BeltPortReceiverInternalVariant" => const_mapping![Mapping::redirect("BeltPortReceiver")],

        //rotating
        "RotatorOneQuadInternalVariant" => const_mapping![
            Mapping::redirect("Rotator1Quad"),
            Mapping::new("Rotator1QuadPlatform90CC", Vec3::new(0.0, 0.05, 0.0))
        ],
        "RotatorOneQuadCCWInternalVariant" => const_mapping![
            Mapping::redirect("Rotator1Quad"),
            Mapping::new("Rotator1QuadPlatform90CW", Vec3::new(0.0, 0.05, 0.0))
        ],
        "RotatorHalfInternalVariant" => const_mapping![
            Mapping::redirect("Rotator1Quad"),
            Mapping::new("Rotator1QuadPlatform180", Vec3::new(0.0, 0.05, 0.0))
        ],

        //processing
        "CutterDefaultInternalVariant" => const_mapping![Mapping::redirect("CutterStatic_Fixed")],
        "StackerDefaultInternalVariant" => const_mapping![Mapping::redirect("StackerSolid")],
        "PainterDefaultInternalVariant" => const_mapping![Mapping::redirect("PainterBasin")],
        "MixerDefaultInternalVariant" => const_mapping![Mapping::redirect("MixerFoundation")],
        "CutterHalfInternalVariant" => const_mapping![Mapping::redirect("HalfCutter")],
        "PinPusherDefaultInternalVariant" => const_mapping![
            Mapping::redirect("PinPusher"),
            Mapping::redirect("PinPusherRotator1"),
            Mapping::new("PinPusherClampR", Vec3::new(0.0, 0.16, 0.0)),
            Mapping::new("PinPusherClampL", Vec3::new(0.0, 0.15, 0.0))
        ],

        //pipes normal
        "PipeLeftInternalVariant" => const_mapping![Mapping::redirect("PipeLeftGlas")],
        "PipeRightInternalVariant" => const_mapping![Mapping::redirect("PipeRightGlas")],
        "PipeCrossInternalVariant" => const_mapping![Mapping::redirect("PipeCrossJunctionGlas")],
        "PipeJunctionInternalVariant" => const_mapping![Mapping::redirect("PipeJunctionGlas")],
        //pipes up
        "PipeUpForwardInternalVariant" => const_mapping![Mapping::redirect("Pipe1UpForwardGlas")],
        "PipeUpBackwardInternalVariant" => const_mapping![Mapping::redirect("Pipe1UpBackwardGlas")],
        "PipeUpLeftInternalVariant" => const_mapping![Mapping::redirect("Pipe1UpLeftBlueprint")],
        "PipeUpRightInternalVariant" => const_mapping![Mapping::redirect("Pipe1UpRightBlueprint")],
        //pipes down
        "PipeDownForwardInternalVariant" => const_mapping![Mapping::redirect("Pipe1DownGlas")],
        "PipeDownBackwardInternalVariant" => {
            const_mapping![Mapping::redirect("Pipe1DownBackwardGlas")]
        }
        "PipeDownRightInternalVariant" => const_mapping![Mapping::redirect("Pipe1DownRightGlas")],
        "PipeDownLeftInternalVariant" => const_mapping![Mapping::redirect("Pipe1DownLeftGlas")],

        // Support Buildings
        "LabelDefaultInternalVariant" => const_mapping![Mapping::redirect("LabelSupport")],
        "FluidStorageDefaultInternalVariant" => {
            const_mapping![Mapping::redirect("PaintTankFoundation")]
        }
        "StorageDefaultInternalVariant" => const_mapping![Mapping::redirect("StorageSolid")],
        "SandboxFluidProducerDefaultInternalVariant" => {
            const_mapping![Mapping::redirect("SandboxIFluidProducer")]
        }
        _ => return None,
    })
}
