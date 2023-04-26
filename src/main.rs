use crate::blueprint::Blueprint;
use crate::tweaks::ModelLoader;
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, LogLevel, Verbosity};
use lazy_static::lazy_static;
use log::{error, info, set_logger, set_max_level, LevelFilter, Log, Metadata, Record};
use std::io::{stderr, Write};
use std::path::PathBuf;
use std::process::exit;
use std::time::Instant;

mod blueprint;
mod render;
mod tweaks;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    input_file: Option<String>,
    #[arg(short, long, default_value = "./models")]
    model_dir: PathBuf,
    #[arg(short, long)]
    tweaks: Option<PathBuf>,
    #[arg(short, long)]
    out_file: Option<PathBuf>,
    #[clap(flatten)]
    verbose: Verbosity<InfoLevel>,
    #[arg(long, default_value = "1980")]
    width: u32,
    #[arg(long, default_value = "1080")]
    height: u32,
    #[arg(short, long, default_value = "4")]
    force_multisample: u32,
}

lazy_static! {
    static ref ARGS: Args = Args::parse();
}

fn main() {
    let program_start_time = Instant::now();
    let logger = Box::new(ApplicationLogger {
        verbosity: ARGS.verbose.clone(),
        start_time: program_start_time,
    });

    set_logger(Box::leak(logger)).expect("no other logger has been registered");
    set_max_level(LevelFilter::Trace);

    if !ARGS.model_dir.is_dir() {
        error!(
            "expected model path {} to be a directory.",
            ARGS.model_dir.display()
        );
        error!("You can use '--model-dir <path>' to specify a different path");
        exit(1);
    }

    let parse_start_time = Instant::now();
    let blueprint = match &ARGS.input_file {
        Some(file) => Blueprint::read_from_file(file),
        None => Blueprint::read_from_stdin(),
    };
    info!("Blueprint parse duration: {:?}", parse_start_time.elapsed());

    let mut loader = ModelLoader::from_config_or_default(ARGS.tweaks.as_ref(), &ARGS.model_dir);

    // Preloading the models just makes it so that the model load time is not added to the outputted total render time
    let model_preload_start_time = Instant::now();
    for entry in &*blueprint {
        loader.load_model(entry.internal_name());
    }
    info!(
        "Preloaded model .obj files used by blueprint in {:?}",
        model_preload_start_time.elapsed()
    );

    if let Err(err) = render::perform_render(&blueprint, &mut loader) {
        error!("Encountered rendering error: {}", err);
        exit(1);
    }

    let (resolved, total) = loader.load_counts();

    info!("Resolved a total of {}/{} models", resolved, total);
    info!("Total duration: {:?}", program_start_time.elapsed());
}

pub struct ApplicationLogger<T: LogLevel> {
    verbosity: Verbosity<T>,
    start_time: Instant,
}

impl<T: LogLevel + Sync + Send> Log for ApplicationLogger<T> {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.verbosity.log_level_filter()
    }

    fn log(&self, record: &Record) {
        match record.module_path() {
            Some(module) => eprintln!(
                "[{}, {:?}][{}]: {}",
                record.level(),
                self.start_time.elapsed(),
                module,
                record.args()
            ),
            None => eprintln!(
                "[{}, {:?}]: {}",
                record.level(),
                self.start_time.elapsed(),
                record.args()
            ),
        }
    }

    fn flush(&self) {
        let _ = stderr().flush();
    }
}
