# SpaceSnifferX

A cross-platform disk space analyzer with real-time treemap visualization, inspired by [SpaceSniffer](http://www.uderzo.it/main_products/space_sniffer/).

## Features

- **Treemap visualization** — directory contents rendered as nested rectangles proportional to file size (squarified treemap algorithm)
- **Real-time scanning** — rectangles appear and grow as the scan progresses, no waiting for a progress bar to finish
- **Directory drill-down** — click any rectangle to zoom into that directory; breadcrumb navigation to go back
- **Context menu** — right-click any file/folder to **open in file manager** or **delete** (with confirmation)
- **Color coding** — directories (blue), documents (red), code (green), images (purple), video (dark red), audio (blue-purple), archives (yellow), executables (gray), others (auto-hashed)
- **Cross-platform** — Windows, macOS, Linux
- **Embedded CJK font** — automatically loads system CJK fonts for Chinese/Japanese/Korean paths
- **SVG window icon** — vector icon compiled into the binary

## Screenshot

```
┌─────────────────────────────────────────────────┐
│ SpaceSnifferX │ [/home/user] [Scan] [Browse...] │
├─────────────────────────────────────────────────┤
│ [■ Dir1    ] [■ Dir2        ] [■ Dir3 ]        │
│ [6.2 GB   ] [■ Dir2.1     ]  [2.1GB ] [■  ...] │
│             [■ Dir2.2 ] [■■■■■■■■File1■■■■■■■ ] │
│             [1.8GB   ] [  3.5 GB              ] │
├─────────────────────────────────────────────────┤
│ Total: 45.2 GB │ ■Dir ■Doc ■Code ■Img ■Vid ..  │
└─────────────────────────────────────────────────┘
```

## Building

Requirements:
- Rust 1.80+
- Cargo

```bash
git clone https://github.com/user/spacesnifferx.git
cd spacesnifferx
cargo build --release
```

The binary will be at `target/release/spacesnifferx` (or `.exe` on Windows).

## Usage

1. Launch the app — it starts idle showing a path input
2. Type or **Browse...** to select a directory
3. Click **Scan** — rectangles appear and grow in real time
4. **Left-click** a directory rectangle to drill into it
5. **Left-click** empty space to go up one level
6. **Right-click** any rectangle for options:
   - **Open file location** — reveals the file in your system file manager
   - **Delete** — removes the file/folder (with confirmation dialog)
7. Use the **breadcrumb bar** at the top to jump levels quickly

### Recommended scan targets

For a quick demo, scan a project folder or `~/Documents`. Scanning your entire home directory may take longer but works fine.

## Project Structure

```
spacesnifferx/
├── Cargo.toml
├── build.rs                  # (unused, kept for future slint backend)
├── icon.svg                  # vector window icon
├── ui/
│   └── main.slint            # Slint UI definition (experimental)
├── src/
│   ├── main.rs               # entry point, CJK font, icon loading
│   ├── app.rs                # UI state, egui rendering, navigation
│   ├── scanner.rs            # recursive directory scanner
│   └── treemap.rs            # squarified treemap layout algorithm
```

### Key algorithms

- **Squarified Treemap** (Bruls, Huizing, van Wijk) — produces rectangles with aspect ratios as close to 1:1 as possible
- **Incremental scan** — scanner thread sends partial tree snapshots every 150ms for real-time rendering

## Dependencies

| Crate | Purpose |
|-------|---------|
| `eframe` / `egui` | GUI framework (immediate mode) |
| `rfd` | Native file/folder picker dialog |
| `resvg` / `tiny-skia` | SVG icon rendering |
| `walkdir` | (reserved for future optimized scanning) |

## License

MIT
