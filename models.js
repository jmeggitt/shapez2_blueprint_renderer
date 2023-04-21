"use strict";

import {OBJLoader} from "three/addons/loaders/OBJLoader.js";
import fs from "fs";
import path from "path";
import sanitize from "sanitize-filename";

function mapInternalName(name) {
    const MAPPINGS = {
        //belts
        "BeltDefaultForwardInternalVariant": "Belt_Straight",
        "BeltDefaultRightInternalVariant": "Belt_90_R",
        "BeltDefaultLeftInternalVariant": "Belt_90_L",

        //vertical
        "Lift1UpBackwardInternalVariant": "Lift1UpBackwards",

        //belts special
        "SplitterTShapeInternalVariant": "Splitter2to1T",
        "MergerTShapeInternalVariant": "Merger2to1T",
        "BeltPortSenderInternalVariant": "BeltPortSender",
        "BeltPortReceiverInternalVariant": "BeltPortReceiver",

        //rotating
        "RotatorOneQuadInternalVariant": "Rotator1QuadPlatform90CC", // arrows onlu
        "RotatorOneQuadCCWInternalVariant": "Rotator1QuadPlatform90CW", // ^
        "RotatorHalfInternalVariant": "Rotator1QuadPlatform180", // ^^

        //processing
        "CutterDefaultInternalVariant": "CutterStatic_Fixed",
        "StackerDefaultInternalVariant": "StackerSolid",
        "PainterDefaultInternalVariant": "PainterBasin",
        "MixerDefaultInternalVariant": "MixerFoundation",
        "CutterHalfInternalVariant": "HalfCutter",

        //pipes normal
        "PipeLeftInternalVariant": "PipeLeftGlas",
        "PipeRightInternalVariant": "PipeRightGlas",
        "PipeCrossInternalVariant": "PipeCrossJunctionGlas",
        "PipeJunctionInternalVariant": "PipeJunctionGlas",

        //pipes up
        "PipeUpForwardInternalVariant": "Pipe1UpForwardGlas",
        "PipeUpBackwardInternalVariant": "Pipe1UpBackwardGlas",
        "PipeUpLeftInternalVariant": "Pipe1UpLeftBlueprint",
        "PipeUpRightInternalVariant": "Pipe1UpRightBlueprint",

        //pipes down
        "PipeDownForwardInternalVariant": "Pipe1DownGlas",
        "PipeDownBackwardInternalVariant": "Pipe1DownBackwardGlas",
        "PipeDownRightInternalVariant": "Pipe1DownRightGlas",
        "PipeDownLeftInternalVariant": "Pipe1DownLeftGlas",

        // Support Buildings
        "LabelDefaultInternalVariant": "LabelSupport",
        "FluidStorageDefaultInternalVariant": "PaintTankFoundation",
        "StorageDefaultInternalVariant": "StorageSolid",
        "SandboxFluidProducerDefaultInternalVariant": "SandboxIFluidProducer",
    };

    return MAPPINGS[name] || name;
}

function* possibleModelNames(baseName) {
    const SUFFIXES = ["InternalVariant", "Default"];
    yield mapInternalName(baseName);

    searchLoop: while (true) {
        for (const suffix of SUFFIXES) {
            if (baseName.endsWith(suffix)) {
                baseName = baseName.substring(0, baseName.length - suffix.length);
                yield mapInternalName(baseName);

                continue searchLoop;
            }
        }

        return;
    }
}

export class ModelLoader {

    constructor(modelDir) {
        this.modelDir = modelDir;
        this.loader = new OBJLoader();

        this.inProgress = {};
    }

    #attemptLoadFile(name) {
        const filePath = path.join(this.modelDir, `${sanitize(name)}.obj`);

        if (this.inProgress.hasOwnProperty(filePath)) {
            return this.inProgress[filePath];
        }

        const objLoader = this.loader;
        const loadPromise = new Promise((resolve, reject) => {
            if (!fs.existsSync(filePath)) {
                return resolve(null);
            }

            fs.readFile(filePath, "utf8", function (err, data) {
                if (err) return reject(err);
                resolve(objLoader.parse(data));
            });
        });

        this.inProgress[filePath] = loadPromise;
        return loadPromise;
    }

    async load(baseName) {
        for (const testName of possibleModelNames(baseName)) {
            const model = await this.#attemptLoadFile(testName);

            if (model !== null) {
                return model;
            }
        }

        console.log(`Missing model for ${baseName}`);
        return null;
    }
}
