# Kifla

Lightweight desktop application for processing textures.

The idea is simple: load a texture, apply one or more operations, preview the result, and save it. The tool is intended as a dedicated workbench for building and using custom texture processing utilities.

The UI consists of four main areas:

* A top menu bar containing file actions and available tools.
* A history panel showing the original texture and all applied operations.
* A tool settings panel used to configure the currently selected operation.
* A central preview area.

The application is designed around full-image operations rather than paint-style editing. Only one tool is active at a time.

Each operation is stored as a history entry containing the operation type and its parameters. The final result is produced by replaying the operation stack from the original texture, providing a simple non-destructive workflow. Operations can be modified, enabled, disabled, removed, or reordered at any time.

Image processing is performed on the CPU for simplicity. The architecture should make it possible to move individual operations to GPU shaders in the future without changing the overall workflow or user interface.

Technology stack:

* Rust
* egui
* eframe
* image
* rfd

The goal of the project is to provide a simple and extensible tool for texture processing.
