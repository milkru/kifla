# kifla User Guide

kifla is a small desktop app for tinkering with textures, with a soft spot for
making them seamless and tileable. You open an image, stack up **modifiers**,
watch the preview update live, and save the result.

Everything is **nondestructive**: your original is never altered. Each modifier
is replayed from the original every time, so you can tweak, reorder, disable, or
remove any of them at any point without losing quality.

---

## Getting started

1. **Open an image.** Use `File → Open` (`Ctrl+O`), or just drag a file onto the
   window. Supported inputs: PNG, JPEG, BMP, TGA, TIFF, and WebP.
2. **Add a modifier.** Click **Add…** in the left panel (`Ctrl+A`), or pick one
   from the **Transform** or **Image** menus.
3. **Adjust its settings.** The preview updates as you go.
4. **Save.** Use `File → Save` (`Ctrl+S`) or `Save As…` (`Ctrl+Shift+S`).

> Adding a modifier does nothing until you touch its settings. Every modifier's
> defaults do nothing, so the stack stays predictable: nothing changes until you
> mean it to.

---

## The modifier stack

The left panel is your stack. Modifiers apply **top to bottom**: the output of
each feeds into the next.

Each row has:

- **Eye toggle:** enable or disable the modifier. A disabled modifier is skipped
  entirely (and won't count as an unsaved change).
- **Name:** drag it up or down to reorder the stack. The rows animate into place.
- **Reset:** appears only when a modifier differs from its defaults, and resets
  it to default values.
- **Remove:** deletes the modifier from the stack.

Modifiers with settings have a collapsible body; click the row to expand or
collapse it.

### Reusing a stack

You can save your whole chain of modifiers and apply it to other images:

- **`File → Export Modifier Stack…`** writes the stack to a `.kstack` file.
- **`File → Import Modifier Stack…`** loads one onto the current image.

This is handy for applying a consistent look or the same tiling treatment across
a set of textures.

---

## The preview

| Action | How |
| --- | --- |
| **Pan** | Drag with the mouse |
| **Zoom** | Scroll wheel (zooms toward the cursor) |
| **Recenter / fit** | `Ctrl+R` |
| **Compare with original** | Hold `Tab` (or `View → Show Original`) |
| **Tile preview** | `Ctrl+T` (repeats the image across the canvas) |
| **Measure a distance** | `Ctrl`+drag (shows the length in pixels) |

The canvas has **rulers** along the top and left edges and crosshair guides that
follow your cursor. The **status bar** at the bottom shows the pixel coordinate
and RGBA value under the cursor, plus the current zoom level.

**Tile preview** is the key tool for seamless work: toggle it on to see your
texture repeated edge to edge so you can spot visible seams instantly.

---

## Making a texture seamless

kifla's tiling tools live under the **Transform** menu. A typical workflow:

1. Turn on **Tile Preview** (`Ctrl+T`) so you can see the seams.
2. Add **Offset** and shift by roughly half the width or height. This wraps the
   image and brings the previously hidden edge seams into the middle, where
   they're easy to see and fix.
3. Add **Blend** to heal the seams: raise **Overlap X / Y** to blend the wrapped
   edges together, and tune **Edge Falloff** for how sharp the transition is.
4. Keep checking the tile preview until the repeat looks clean.

Related tiling modifiers: **Repeat** (tile the image into one texture) and the
tiling aware **Rotate** and **Skew** (they wrap around instead of leaving empty
corners).

---

## Modifier reference

### Transform

| Modifier | What it does | Controls |
| --- | --- | --- |
| **Flip Horizontal / Vertical** | Mirror the image | None |
| **Rotate 90° CW / CCW** | Quarter turn rotation (swaps dimensions) | None |
| **Offset** | Shift the image, wrapping at the edges | X, Y (pixels) |
| **Repeat** | Tile the image into one texture | X, Y (1 to 32) |
| **Blend** | Heal seams by blending the wrapped edges together | Edge Falloff, Overlap X, Overlap Y |
| **Rotate** | Free angle rotation, tiling aware | Angle (up to 45° either way) |
| **Skew** | Slant horizontally / vertically, tiling aware | Horizontal, Vertical (up to 45° either way) |
| **Resize** | Change dimensions | Width, Height, Sampling (see below) |
| **Crop** | Cut out a rectangle | X, Y, Width, Height |

**Resize sampling modes:**

- **Nearest:** hard pixels, no blending (good for pixel art).
- **Bilinear / Bicubic / Lanczos:** increasingly sharp smooth filtering.
- **Min / Max:** keep the darkest or brightest pixel in each block (useful for
  masks); each has a **Threshold**.
- **Detail Preserving:** a downscale that keeps thin features like seams; has a
  **Detail** strength.

### Image: Tone

| Modifier | What it does | Controls |
| --- | --- | --- |
| **Brightness / Contrast** | Basic tonal adjustment | Brightness, Contrast |
| **Levels** | Remap the tonal range | Input black/white, Gamma, Output black/white |
| **Curves** | Freeform tone curve | Interactive editor (see below) |
| **Exposure** | Photographic exposure | Exposure (stops), Offset, Gamma |

The **Curves** editor: click an empty spot to add a point, drag points to shape
the curve, and right click a point to delete it (the two endpoints stay put).

### Image: Color

| Modifier | What it does | Controls |
| --- | --- | --- |
| **Hue / Saturation** | Shift hue, saturation, lightness | Hue (°), Saturation, Lightness |
| **Vibrance** | Saturation that boosts muted colors more than already saturated ones | Vibrance, Saturation |
| **Color Balance** | Tint shadows / midtones / highlights | Cyan to Red, Magenta to Green, Yellow to Blue, per range |
| **Black & White** | Custom grayscale conversion | Red / Green / Blue weights, Amount |
| **Channel Mixer** | Build each output channel from the inputs | R/G/B mix per output channel |

### Image: Stylize

| Modifier | What it does | Controls |
| --- | --- | --- |
| **Posterize** | Reduce to N tonal steps | Levels (2 to 256) |
| **Threshold** | Convert to pure black & white at a cutoff | Threshold, Amount |
| **Selective Color** | Adjust a specific color family | Family, Cyan/Magenta/Yellow/Black |
| **Indexed Color** | Quantize to a limited palette | Colors (2 to 256), Dither, Amount |
| **Invert** | Invert colors | None |

**Indexed Color** picks an adaptive palette automatically and can apply blue
noise **dithering** to smooth banding when you drop to few colors.

### Image: Light

| Modifier | What it does | Controls |
| --- | --- | --- |
| **Shadows / Highlights** | Lift shadows or recover highlights independently | Shadows, Highlights |
| **Lighting Normalization** | Even out uneven lighting across the image | Amount |

---

## Interaction tips

- **Fine tune any slider or number:** hover it and `Ctrl`+scroll to nudge the
  value by a small step.
- **Cycle a dropdown:** hover the closed dropdown and `Ctrl`+scroll to step
  through its options.
- Slider drags and typed values each collapse into a **single undo step**.

---

## Saving

- **`Ctrl+S`**: save to the current file.
- **`Ctrl+Shift+S`**: Save As, choosing the format: **PNG, JPEG, BMP, TGA**, or
  **ICO**. ICO is written as an icon containing multiple sizes (256 down to
  16 px).

---

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+O` | Open image |
| `Ctrl+S` | Save |
| `Ctrl+Shift+S` | Save As |
| `Ctrl+W` | Close image |
| `Ctrl+Q` | Quit |
| `Ctrl+A` | Add a modifier |
| `Ctrl+Z` / `Ctrl+Y` | Undo / Redo |
| `Ctrl+R` | Recenter the view |
| `Ctrl+T` | Toggle tile preview |
| `Tab` (hold) | Show the original |
| `Ctrl`+drag | Measure a distance |
| `F1` | About |
