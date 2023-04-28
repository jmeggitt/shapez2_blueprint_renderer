import {waitOnChildStdout, wrapExecutable} from "./exec.ts";
import {MODEL_DIR} from "./config.js";

export enum ImageFilter {
    Nearest = "nearest",
    Linear = "linear",
    Cubic = "cubic",
    Gaussian = "gaussian",
    Lanczos3 = "lanczos3",
}

export enum LogLevel {
    Debug = "debug",
    Info = "info",
    Warn = "warn",
    Error = "error",
    Off = "off",
}

export type RenderOptions = {
    /**
     * Width and height of the output image
     */
    size?: [number, number],
    /**
     * Super Sample Anti-Alias
     */
    ssaa?: number,
    /**
     * Filter to use when resizing an image from the SSAA size to the desired size
     */
    ssaaSampler?: ImageFilter,
    /**
     * If set, the program will save an image instead of returning the image data. The image format is determined from
     * the file extension. When selected, an empty buffer will be returned.
     */
    outFile?: string,
    /**
     * The path to the models used for rendering the image
     */
    modelDir?: string,
    /**
     * Changes the verbosity of logging.
     */
    verbosity?: LogLevel,
    /**
     * This option can be used to manually enforce whether to attempt to create a headless display to render to. If not
     * specified, this will be inferred from the current platform.
     */
    headless?: boolean,
};

/**
 * Renders a blueprint and returns the binary data for the resulting PNG image.
 *
 * @param blueprint The blueprint string
 * @param options Render configuration options
 */
export function render(blueprint: string | Buffer, options?: RenderOptions): Promise<Buffer> {
    const optionsWithDefaults: RenderOptions = options ?? {};
    optionsWithDefaults.modelDir = options?.modelDir ?? MODEL_DIR;
    optionsWithDefaults.verbosity = options?.verbosity ?? LogLevel.Warn;

    const args = optionsToProgramArgs(optionsWithDefaults);

    const ssaa = options?.ssaa ?? 1;
    const [width, height] = options?.size ?? [1980, 1080];

    const [program, wrapperArgs] = wrapExecutable([width * ssaa, height * ssaa], options?.headless);
    args.push(...wrapperArgs);

    return waitOnChildStdout(blueprint, program, args);
}


function verbosityArgs(logLevel?: LogLevel): Array<string> {
    return {
        [LogLevel.Debug]: ["-v"],
        [LogLevel.Info]: [],
        [LogLevel.Warn]: ["-q"],
        [LogLevel.Error]: ["-qq"],
        [LogLevel.Off]: ["-qqq"],
    }[logLevel];
}

function optionsToProgramArgs(options: RenderOptions): Array<string> {
    const OPTION_TO_ARGS = {
        "size": ([width, height]) => ["--width", width.toString(), "--height", height.toString()],
        "ssaa": x => ["--ssaa", x.toString()],
        "ssaaSampler": x => ["--ssaa-sampler", x.toString()],
        "outFile": x => ["--out-file", x],
        "modelDir": x => ["--model-dir", x],
        "verbosity": verbosityArgs,
        "headless": _ => [],
    };

    const args = [];

    for (const [key, value] of Object.entries(options)) {
        if (value === undefined) continue;

        if (!OPTION_TO_ARGS.hasOwnProperty(key)) {
            throw new Error(`Unknown option "${key}" for RenderOptions`);
        }

        args.push(...OPTION_TO_ARGS[key](value));
    }

    return args;
}


