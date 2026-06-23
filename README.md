# kifla

A small desktop app for tinkering with textures.

Open an image (or drop one in), stack up some modifiers, watch the preview update live, and save the result. Everything is non-destructive - each modifier is replayed from the original, so you can tweak, toggle, or remove any of them at any time. Hold the compare key (Tab) to peek at the original, and the preview has pan/zoom, recenter, rulers and a cursor readout.

## What it does

- **Transform** - flip, rotate, resize (with a few sampling modes, including min/max-luminance for masks and a detail-preserving downscale that keeps thin seams)
- **Image adjustments** - brightness/contrast, levels, curves, exposure, hue/saturation, vibrance, color balance, black & white, channel mixer, posterize, threshold, selective color, invert, shadows/highlights
- **Save** to PNG, JPEG, BMP, TGA, or multi-resolution ICO

Adding a modifier does nothing until you touch its settings - defaults are no-ops, so the stack stays predictable.

## Under the hood

Processing runs on the CPU, parallelized across all cores. Each modifier is a self-contained type implementing a small `Modifier` trait, so adding a new one is mostly: write a file, register it in a menu group. The architecture leaves room to move individual modifiers onto the GPU later without changing the workflow.

## Built with

Rust · egui · eframe · image · rayon · rfd
