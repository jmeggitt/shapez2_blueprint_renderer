"use strict";

import {gunzip} from "zlib";

import {PerspectiveCamera, Scene, Mesh, Vector3} from "three";
import {ModelLoader} from "./models.js";
import {buildRenderer, contextBufferToImage} from "./render.js";
import fs from "fs";

function asyncGunzip(binaryData) {
    return new Promise((resolve, reject) => {
        gunzip(binaryData, (err, buffer) => {
            if (err) return reject(err);
            resolve(buffer);
        });
    });
}

async function readBase64GzipJson(data) {
    const compressed = Buffer.from(data, "base64");
    const binary = await asyncGunzip(compressed);
    return JSON.parse(binary);
}

function splitBlueprint(blueprint) {
    const SEPARATOR = "-";
    const HEADER = "SHAPEZ2" + SEPARATOR;
    const FOOTER = "$";

    if (!blueprint.startsWith(HEADER) || !blueprint.endsWith(FOOTER)) {
        throw new Error("value is not valid blueprint");
    }

    const versionSeparatorIndex = blueprint.indexOf(SEPARATOR, HEADER.length);
    if (versionSeparatorIndex === -1) {
        throw new Error("missing version separator in blueprint");
    }

    const versionString = blueprint.substring(HEADER.length, versionSeparatorIndex);
    const dataString = blueprint.substring(versionSeparatorIndex + 1);

    return [versionString, dataString];
}


async function renderBlueprintVersion1(data, {modelDir, width, height}) {
    const blueprint = await readBase64GzipJson(data);
    // TODO: Verify blueprint meets format requirements and isn't just malicious JSON
    console.log(blueprint);

    const aspect_ratio = width / height;
    const camera = new PerspectiveCamera(1.2, aspect_ratio, 0.1, 1000.0);
    camera.position.set(-10, 10, -10);
    camera.up = new Vector3(0, 1, 0);
    camera.lookAt(new Vector3(0, 0, 0));


    const renderer = buildRenderer({width, height});

    const scene = new Scene();
    const loader = new ModelLoader(modelDir);

    const context = renderer.getContext();
    context.clearColor(1, 0, 0, 1);
    context.clear(context.COLOR_BUFFER_BIT | context.DEPTH_BUFFER_BIT);

    for (const entry of blueprint["BP"]["Entries"]) {
        const model = await loader.load(entry.T);
        if (model === null) continue;

        // Create new mesh, so we don't overwrite the position when used multiple times
        const {geometry, material} = model.children[0];
        const mesh = new Mesh(geometry, material);

        mesh.position.x = entry["X"];
        mesh.position.y = entry["L"];
        mesh.position.z = entry["Y"];

        mesh.rotation.set(new Vector3(0, entry["R"] * Math.PI / 2, 0));

        scene.add(model);
    }

    renderer.render(scene, camera);

    return contextBufferToImage(renderer.getContext());
}


function loadDefaultOptions(options) {
    const DEFAULT_OPTIONS = {
        modelDir: "./models",
        width: 1280,
        height: 720,
    };

    return {...DEFAULT_OPTIONS, ...options};
}

// At the moment, there is only one blueprint version, but in theory,
const SUPPORTED_VERSIONS = {
    "1": renderBlueprintVersion1,
};

export function renderBlueprint(blueprint, options) {
    const [version, data] = splitBlueprint(blueprint);

    if (!SUPPORTED_VERSIONS.hasOwnProperty(version)) {
        throw new Error("blueprint version is unsupported");
    }

    const optionsWithDefaults = loadDefaultOptions(options || {})
    return (SUPPORTED_VERSIONS[version])(data, optionsWithDefaults);
}


// Temporary debugging code
const testInput = "SHAPEZ2-1-H4sIABfwO2QA/6ySTQuCQBCG/8ucV3A2uuzRLAg2CDUposNCawmyK9uGB/G/J3oxzA+wy5xmHp55Z0qIgaHrrgl4R2AlbJU1qXwBu5ZwBuYSuDSVNzUAtiIQAQNPZtaXiXhndqdNIcx9r6w0SmSxMKlQFghs6j6oSAeEfVCYZ6mtJzHSNBhn0H/J0DkyfJyBPYbbMg7SPKSJwqfI5RwNB0cX4jKxc0y+MHTJkX6R2qVopHEiFud/Njh0bp4mFk/5dDLd78UlKg4OkRoXXxdqgnOrqo8AAwD/blmSbAMAAA==$";

const image = await renderBlueprint(testInput);
await image.pack().pipe(fs.createWriteStream("out.png"));
