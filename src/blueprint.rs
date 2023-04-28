use base64::prelude::BASE64_STANDARD;
use base64::read::DecoderReader;
use flate2::read::GzDecoder;
use log::error;
use nalgebra_glm::Vec3;
use num_traits::FloatConst;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io;
use std::io::ErrorKind::InvalidData;
use std::io::{stdin, BufReader, Error, Read};
use std::ops::Deref;
use std::path::Path;
use std::process::exit;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct Blueprint {
    v: i32,
    bp: BlueprintEntries,
}

impl Blueprint {
    pub fn read_from_file<P: AsRef<Path>>(path: P) -> Self {
        let mut file = match File::open(path) {
            Ok(v) => BufReader::new(v),
            Err(e) => {
                error!("unable to open input file: {}", e);
                exit(1);
            }
        };

        match Self::decode(&mut file) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse blueprint: {}", e);
                exit(1);
            }
        }
    }

    pub fn read_from_stdin() -> Self {
        let mut stdin = stdin();

        match Self::decode(&mut stdin) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to parse blueprint: {}", e);
                exit(1);
            }
        }
    }

    fn decode<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut data = Vec::new();
        io::copy(reader, &mut data)?;

        let utf8 = String::from_utf8(data)
            .map_err(|_| Error::new(InvalidData, "Blueprint must be utf-8"))?;

        let mut trimmed = utf8.as_str().trim();
        trimmed = trimmed.strip_prefix("SHAPEZ2-1-").ok_or_else(|| {
            Error::new(InvalidData, "Expected blueprint to start with 'SHAPEZ2-1-'")
        })?;
        trimmed = trimmed
            .strip_suffix('$')
            .ok_or_else(|| Error::new(InvalidData, "Expected blueprint to end with '$'"))?;

        let mut reader = trimmed.as_bytes();
        let decoder = DecoderReader::new(&mut reader, &BASE64_STANDARD);
        let deflate = GzDecoder::new(decoder);

        serde_json::from_reader(deflate)
            .map_err(|err| {
                error!("Error occurred during blueprint read pipeline (Base64 decode -> Gunzip -> Json parse): {}", err);
                Error::new(InvalidData, "Unable to decode blueprint data")
            })
    }
}

impl Deref for Blueprint {
    type Target = [BlueprintEntry];

    fn deref(&self) -> &Self::Target {
        &self.bp.entries
    }
}

#[derive(Serialize, Deserialize)]
pub struct BlueprintEntries {
    #[serde(rename = "Entries")]
    entries: Vec<BlueprintEntry>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct BlueprintEntry {
    x: i32,
    y: i32,
    #[serde(rename = "L")]
    layer: i32,
    #[serde(rename = "R")]
    rotation: i32,
    #[serde(rename = "T")]
    internal_name: String,
    #[serde(rename = "C")]
    attached_data: String,
}

impl BlueprintEntry {
    /// Get the position with layer mapped to the Y axis.
    pub fn position(&self) -> Vec3 {
        Vec3::new(self.x as f32, self.layer as f32, self.y as f32)
    }

    /// Get the rotation of this entry in radians
    pub fn rotation(&self) -> f32 {
        self.rotation as f32 * f32::PI() / 2.0
    }

    pub fn internal_name(&self) -> &str {
        &self.internal_name
    }
}
