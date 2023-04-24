use crate::tweaks::ModelLoader;
use base64::prelude::BASE64_STANDARD;
use base64::read::DecoderReader;
use clap::Parser;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::ErrorKind::InvalidData;
use std::io::{self, stdin, BufReader, Error, Read};
use std::path::{Path, PathBuf};
use std::process::exit;

// mod render_old;
mod tweaks;
// mod render;
// mod render_old;
mod render;
// mod render;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    input_file: Option<String>,
    #[arg(short, long, default_value = "./models")]
    model_dir: PathBuf,
    #[arg(short, long)]
    tweaks: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    println!("{:?}", &args);

    if !args.model_dir.is_dir() {
        println!(
            "Error: expected model path {} to be a directory.",
            args.model_dir.display()
        );
        println!("You can use '--model-dir <path>' to specify a different path");
        exit(1);
    }

    let blueprint = match &args.input_file {
        Some(file) => read_from_file(file),
        None => read_from_stdin(),
    };

    let mut loader = ModelLoader::from_config_or_default(args.tweaks.as_ref(), args.model_dir);

    if let Err(err) = render::perform_render(&blueprint.bp.entries, &mut loader) {
        println!("Encountered rendering error: {}", err);
        exit(1);
    }

    // if let Err(err) = render_old::perform_render(&blueprint.bp.entries, &mut loader) {
    //     println!("Encountered rendering error: {}", err);
    //     exit(1);
    // }

    let (resolved, total) = loader.load_counts();
    println!("Resolved a total of {}/{} models", resolved, total);


    // render::setup_opengl(1980, 1080);
    // let (_, gl) = render::setup_opengl(1000, 1000);
}

fn read_from_file<P: AsRef<Path>>(path: P) -> BlueprintData {
    let mut file = match File::open(path) {
        Ok(v) => BufReader::new(v),
        Err(e) => {
            println!("Error: unable to open file: {}", e);
            exit(1);
        }
    };

    match decode_blueprint(&mut file) {
        Ok(v) => v,
        Err(e) => {
            println!("Failed to parse blueprint: {}", e);
            exit(1);
        }
    }
}

fn read_from_stdin() -> BlueprintData {
    let mut stdin = stdin();

    match decode_blueprint(&mut stdin) {
        Ok(v) => v,
        Err(e) => {
            println!("Failed to parse blueprint: {}", e);
            exit(1);
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct BlueprintData {
    v: i32,
    bp: BlueprintEntries,
}

#[derive(Serialize, Deserialize)]
struct BlueprintEntries {
    #[serde(rename = "Entries")]
    entries: Vec<BlueprintEntry>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct BlueprintEntry {
    x: i32,
    y: i32,
    l: i32,
    r: i32,
    #[serde(rename = "T")]
    internal_name: String,
    c: String,
}

fn decode_blueprint<R: Read>(reader: &mut R) -> io::Result<BlueprintData> {
    let mut data = Vec::new();
    io::copy(reader, &mut data)?;

    let utf8 =
        String::from_utf8(data).map_err(|_| Error::new(InvalidData, "Blueprint must be utf-8"))?;

    let mut trimmed = utf8.as_str();
    trimmed = trimmed
        .strip_prefix("SHAPEZ2-1-")
        .ok_or_else(|| Error::new(InvalidData, "Expected blueprint to start with 'SHAPEZ2-1-'"))?;
    trimmed = trimmed
        .strip_suffix('$')
        .ok_or_else(|| Error::new(InvalidData, "Expected blueprint to end with '$'"))?;

    let mut reader = trimmed.as_bytes();
    let decoder = DecoderReader::new(&mut reader, &BASE64_STANDARD);
    let deflate = GzDecoder::new(decoder);

    serde_json::from_reader(deflate)
        .map_err(|_| Error::new(InvalidData, "Unable to decode blueprint data"))
}
