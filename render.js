"use strict";

import {WebGLRenderer, WebGLRenderTarget, LinearFilter, NearestFilter, RGBAFormat, UnsignedByteType} from "three";
import gl from "gl";
import {PNG} from "pngjs";

export function buildRenderer({width, height}) {
    const dummyCanvas = {
        width,
        height,
        addEventListener: _ => {},
        removeEventListener: _ => {},
    };

    const renderer = new WebGLRenderer({
        canvas: dummyCanvas,
        antialias: true,
        powerPreference: "high-performance",
        context: gl(width, height, {
            preserveDrawingBuffer: true,
        }),
    });

    const target = new WebGLRenderTarget(width, height, {
        minFilter: NearestFilter,
        magFilter: LinearFilter,
        generateMipmaps: true,
        format: RGBAFormat,
        type: UnsignedByteType,
        samples: 8,
    });

    renderer.setRenderTarget(target);
    return renderer;
}

export function contextBufferToImage(context) {
    const width = context.drawingBufferWidth;
    const height = context.drawingBufferHeight;

    const img = new PNG({width, height});
    context.readPixels(0, 0, width, height, context.RGBA, context.UNSIGNED_BYTE, img.data);

    return img;
}


