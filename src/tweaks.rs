use nalgebra_glm::Vec3;
use obj::Obj;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader};
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
                        resolved_objects: HashMap::new(),
                        model_sets: HashMap::new(),
                        model_dir,
                    };
                }
                Err(e) => {
                    eprintln!("Error: failed to read model tweaks file: {}", e);
                    exit(1);
                }
            }
        }

        ModelLoader {
            _config: ModelTweaksConfig::default(),
            resolved_objects: HashMap::new(),
            model_sets: HashMap::new(),
            model_dir,
        }
    }

    fn try_load_object<P: AsRef<Path>>(path: P) -> Option<Obj> {
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
                return None;
            }
        };

        if let Err(e) = obj.load_mtls() {
            eprintln!("Error: failed to load model materials: {}", e);
            return None;
        }

        Some(obj)
    }

    fn find_object(&mut self, mut name: &str) -> Option<Rc<Obj>> {
        if let Some(obj) = self.resolved_objects.get(name) {
            return Some(obj.clone());
        }

        'search: loop {
            if let Some(obj) = Self::try_load_object(format!("{}.obj", name)) {
                let reference_counted = Rc::new(obj);
                self.resolved_objects.insert(name.to_owned(), reference_counted.clone());
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
            None => Vec::from_iter(
                self.find_object(name)
                    .map(|model| Model {
                        model,
                        offset: Vec3::default(),
                    })),
            Some(mappings) => Vec::from_iter(
                mappings.iter()
                    .filter_map(|&Mapping { file, offset }| {
                        Some(Model {
                            model: self.find_object(file)?,
                            offset,
                        })
                    })),
        };


        if model_set.is_empty() {
            println!("Failed to find model for {}; building will not be rendered", name);
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
    fn new(file: &'s str, offset: Vec3) -> Self {
        Mapping { file, offset }
    }

    fn redirect(file: &'s str) -> Self {
        Mapping {
            file,
            offset: Vec3::default(),
        }
    }
}

fn internal_name_mapping_adjustments(internal_name: &str) -> Option<&[Mapping]> {
    Some(match internal_name {
        //belts
        "BeltDefaultForwardInternalVariant" => &[Mapping::redirect("Belt_Straight")],
        "BeltDefaultRightInternalVariant" => &[Mapping::redirect("Belt_90_R")],
        "BeltDefaultLeftInternalVariant" => &[Mapping::redirect("Belt_90_L")],
        //vertical
        "Lift1UpBackwardInternalVariant" => &[Mapping::redirect("Lift1UpBackwards")],
        //belts special
        "SplitterTShapeInternalVariant" => &[Mapping::redirect("Splitter2to1T")],
        "MergerTShapeInternalVariant" => &[Mapping::redirect("Merger2to1T")],
        "BeltPortSenderInternalVariant" => &[Mapping::redirect("BeltPortSender")],
        "BeltPortReceiverInternalVariant" => &[Mapping::redirect("BeltPortReceiver")],

        //rotating
        "RotatorOneQuadInternalVariant" => &[Mapping::redirect("Rotator1Quad"), Mapping::new("Rotator1QuadPlatform90CC", Vec3::new(0.0, 0.05, 0.0))], // arrows only
        "RotatorOneQuadCCWInternalVariant" => &[Mapping::redirect("Rotator1Quad"), Mapping::new("Rotator1QuadPlatform90CW", Vec3::new(0.0, 0.05, 0.0))], // ^
        "RotatorHalfInternalVariant" => &[Mapping::redirect("Rotator1Quad"), Mapping::new("Rotator1QuadPlatform180", Vec3::new(0.0, 0.05, 0.0))], // ^^

        //processing
        "CutterDefaultInternalVariant" => &[Mapping::redirect("CutterStatic_Fixed")],
        "StackerDefaultInternalVariant" => &[Mapping::redirect("StackerSolid")],
        "PainterDefaultInternalVariant" => &[Mapping::redirect("PainterBasin")],
        "MixerDefaultInternalVariant" => &[Mapping::redirect("MixerFoundation")],
        "CutterHalfInternalVariant" => &[Mapping::redirect("HalfCutter")],
        "PinPusherDefaultInternalVariant" => &[Mapping::redirect("PinPusher"), Mapping::redirect("PinPusherRotator1"), Mapping::new("PinPusherClampR", Vec3::new(0.0, 0.16, 0.0)), Mapping::new("PinPusherClampL", Vec3::new(0.0, 0.15, 0.0))],

        //pipes normal
        "PipeLeftInternalVariant" => &[Mapping::redirect("PipeLeftGlas")],
        "PipeRightInternalVariant" => &[Mapping::redirect("PipeRightGlas")],
        "PipeCrossInternalVariant" => &[Mapping::redirect("PipeCrossJunctionGlas")],
        "PipeJunctionInternalVariant" => &[Mapping::redirect("PipeJunctionGlas")],
        //pipes up
        "PipeUpForwardInternalVariant" => &[Mapping::redirect("Pipe1UpForwardGlas")],
        "PipeUpBackwardInternalVariant" => &[Mapping::redirect("Pipe1UpBackwardGlas")],
        "PipeUpLeftInternalVariant" => &[Mapping::redirect("Pipe1UpLeftBlueprint")], // Contains the pump
        "PipeUpRightInternalVariant" => &[Mapping::redirect("Pipe1UpRightBlueprint")], // ^
        //pipes down
        "PipeDownForwardInternalVariant" => &[Mapping::redirect("Pipe1DownGlas")],
        "PipeDownBackwardInternalVariant" => &[Mapping::redirect("Pipe1DownBackwardGlas")],
        "PipeDownRightInternalVariant" => &[Mapping::redirect("Pipe1DownRightGlas")],
        "PipeDownLeftInternalVariant" => &[Mapping::redirect("Pipe1DownLeftGlas")],

        // Support Buildings
        "LabelDefaultInternalVariant" => &[Mapping::redirect("LabelSupport")],
        "FluidStorageDefaultInternalVariant" => &[Mapping::redirect("PaintTankFoundation")],
        "StorageDefaultInternalVariant" => &[Mapping::redirect("StorageSolid")],
        "SandboxFluidProducerDefaultInternalVariant" => &[Mapping::redirect("SandboxIFluidProducer")],
        _ => return None,
    })
}
