use crate::blueprint::Blueprint;
use crate::tweaks::ModelLoader;
use clap::builder::PossibleValue;
use clap::{Parser, ValueEnum};
use clap_verbosity_flag::{InfoLevel, LogLevel, Verbosity};
use image::imageops::FilterType;
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
    /// The file which to read the blueprint from. If an input file is not provided, the blueprint
    /// will instead be read from stdin.
    input_file: Option<PathBuf>,
    /// The directory holding the .obj files representing the various buildings and features within
    /// the game.
    #[arg(short, long, default_value = "./models")]
    model_dir: PathBuf,
    /// The path that the output image will be written to. The image type is detected from the path
    /// extension. If an output file is not provided, the image will instead be written to stdout as
    /// a PNG.
    #[arg(short, long)]
    out_file: Option<PathBuf>,
    #[clap(flatten)]
    verbose: Verbosity<InfoLevel>,
    /// The width of the output image
    #[arg(long, default_value = "1980")]
    width: u32,
    /// The height of the output image
    #[arg(long, default_value = "1080")]
    height: u32,
    /// This argument triggers SSAA on the rendered image. This is provided to allow for
    /// anti-aliasing on systems which do not normally support MSAA. Values over 16 will not
    /// increase the output quality.
    ///
    /// Note: This is applied by increasing the render size and resampling the output. As such,
    /// this is NOT hardware accelerated and is performed on top of any MSAA capabilities the
    /// system has.
    #[arg(long, default_value = "1")]
    ssaa: u32,
    /// The sampler used when resizing a super sampled image to the intended size. This will effect
    /// the final image quality when resizing is required.
    #[arg(long, value_enum, default_value = "linear")]
    ssaa_sampler: ImageFilter,
}

#[derive(Copy, Clone, Debug)]
struct ImageFilter(FilterType);

impl ValueEnum for ImageFilter {
    fn value_variants<'a>() -> &'a [Self] {
        const VARIANTS: &[ImageFilter] = &[
            ImageFilter(FilterType::Nearest),
            ImageFilter(FilterType::Triangle),
            ImageFilter(FilterType::CatmullRom),
            ImageFilter(FilterType::Gaussian),
            ImageFilter(FilterType::Lanczos3),
        ];

        VARIANTS
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        let ImageFilter(filter) = self;

        Some(match filter {
            FilterType::Nearest => {
                PossibleValue::new("nearest").help("Nearest neighbor sampling (fastest)")
            }
            FilterType::Triangle => PossibleValue::new("linear")
                .help("Triangle (linear) sampling (~13x slower than nearest sampling)"),
            FilterType::CatmullRom => PossibleValue::new("cubic")
                .help("Catmull Rom (cubic) sampling (~26x slower than nearest sampling)"),
            FilterType::Gaussian => PossibleValue::new("gaussian")
                .help("Gaussian sampling (~38x slower than nearest sampling)"),
            FilterType::Lanczos3 => PossibleValue::new("lanczos3").help(
                "Lanczos Window 3 sampling (best quality, ~38x slower than nearest sampling)",
            ),
        })
    }
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

    let mut loader = ModelLoader::new(&ARGS.model_dir);

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
