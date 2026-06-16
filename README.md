# Kifla

A small desktop app for tinkering with textures.

Open an image, stack up some edits, watch the preview update live, and save the result. Everything is non-destructive — each edit is replayed from the original, so you can tweak, toggle, or remove any of them at any time.

## What it does

- **Transform** — flip, rotate, resize (with a few sampling modes, including min/max-luminance for masks)
- **Image adjustments** — brightness/contrast, levels, curves, exposure, hue/saturation, vibrance, color balance, black & white, channel mixer, posterize, threshold, selective color, invert, shadows/highlights
- **Save** to PNG, JPEG, BMP, TGA, or multi-resolution ICO

Adding an edit does nothing until you touch its settings — defaults are no-ops, so the stack stays predictable.

## Under the hood

Processing runs on the CPU for now. Each operation is a self-contained type implementing a small `Operation` trait, so adding a new one is mostly: write a file, register it in a menu group. The architecture leaves room to move individual operations onto the GPU later without changing the workflow.

## Built with

Rust · egui · eframe · image · rfd
