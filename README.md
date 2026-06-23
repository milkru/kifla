# kifla

A small desktop app, written in Rust, for tinkering with textures, with a soft spot for making them seamless and tileable.

Open an image (or drop one in), stack up some modifiers, watch the preview update live, and save the result. It does general image adjustments and transforms, and it's especially handy for tiling work: dedicated offset, blend and repeat tools plus a tile preview make it easy to turn a photo into a clean, seamlessly repeating texture. Everything is nondestructive; each modifier is replayed from the original, so you can tweak, toggle, or remove any of them at any time. Hold the compare key (Tab) to peek at the original, and the preview has pan/zoom, recenter, rulers and a cursor readout.

![Example](https://github.com/milkru/data_resources/blob/main/kifla_intro.png "Example Intro")
![Example](https://github.com/milkru/data_resources/blob/main/kifla_work.png "Example Work")

## What it does

- **Transform**: flip, rotate, resize (with a few sampling modes, including min/max luminance for masks and a detail preserving downscale that keeps thin seams)
- **Image adjustments**: brightness/contrast, levels, curves, exposure, hue/saturation, vibrance, color balance, black & white, channel mixer, posterize, threshold, selective color, invert, shadows/highlights
- **Save** to PNG, JPEG, BMP, TGA, or multiresolution ICO

Adding a modifier does nothing until you touch its settings; defaults do nothing, so the stack stays predictable.

## Under the hood

Processing runs entirely on the GPU (wgpu). Each modifier is a small type implementing the `Modifier` trait that supplies one or more shader passes, so adding a new one is mostly: write a file with its shader, register it in a menu group. Most modifiers are a single fragment pass; a few (lighting normalization, indexed color) run multipass or compute pipelines.
