import {spawn} from "node:child_process";
import {EXECUTABLE_PATH} from "./config.ts";


export function wrapExecutable([width, height]: [number, number], headless?: boolean): [string, Array<string>] {
    let useHeadless = headless ?? process.platform !== "win32";

    // If we are using headless mode, we need to wrap it in a virtual screen
    if (useHeadless) {
        return ["xvfb-run", ["-s", `-ac -screen 0 ${width}x${height}x24`, EXECUTABLE_PATH]];
    }

    return [EXECUTABLE_PATH, []];
}


export function waitOnChildStdout(stdin: string | Buffer, executable: string, args: Array<string>): Promise<Buffer> {
    return new Promise((resolve, reject) => {
        const childProcess = spawn(executable, args, {
            "stdio": ["pipe", "pipe", "inherit"],
        });

        let outputBuffer = Buffer.alloc(0);

        // There may be an easier way to handle collecting stdout to a buffer
        childProcess.stdout.on("data", chunk => {
            outputBuffer = Buffer.concat([outputBuffer, chunk])
        });

        childProcess.on("close", code => {
            if (code !== 0) {
                return reject(new Error("Renderer exited with non-zero exit code", {cause: code}));
            }

            resolve(outputBuffer);
        });

        childProcess.on("error", error => {
            reject(new Error("Failed to start child process for renderer", {cause: error}));
        });

        childProcess.stdin.write(stdin);
        childProcess.stdin.end();
    });
}

