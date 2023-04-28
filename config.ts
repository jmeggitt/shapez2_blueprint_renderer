import * as path from "path";

import { fileURLToPath } from 'url';
import { dirname } from 'path';

import { accessSync, constants } from "node:fs";

// Run configuration
const ROOT_DIR = dirname(fileURLToPath(import.meta.url));
export const MODEL_DIR = path.join(ROOT_DIR, "models");


// Executable location
export const RENDER_PROJECT_NAME: string = "shapez2_blueprint_renderer";
export const EXECUTABLE_PATH: string | null = findExecutable([
    // We may want to save space by deleting the build files after compiling the renderer. For this reason, check the
    // project directories first
    ROOT_DIR,
    // Location when building with optimizations (cargo build --release)
    path.join(ROOT_DIR, "target/release"),
    // Location when building for development (cargo build )
    path.join(ROOT_DIR, "target/debug"),
]);

function findExecutable(searchLocations: Array<string>): string | null {
    let executableName: string;

    if (process.platform === "win32") {
        executableName = RENDER_PROJECT_NAME + ".exe";
    } else {
        executableName = RENDER_PROJECT_NAME;
    }

    for (const location of searchLocations) {
        const executablePath = path.join(location, executableName);

        try {
            accessSync(executablePath, constants.X_OK);
            return executablePath;
        } catch (ignored) {}
    }

    throw new Error("Unable to find render executable. Did the project build correctly?")
}
