<div align="center">

# Alaydriem's Stratum

[![Release](https://img.shields.io/github/v/release/alaydriem/levi-stratum?include_prereleases)](https://github.com/alaydriem/levi-stratum/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/alaydriem/levi-stratum/build.yml?branch=master&label=build)](https://github.com/alaydriem/levi-stratum/actions/workflows/build.yml)
[![License](https://img.shields.io/github/license/alaydriem/levi-stratum)](./LICENSE)
![Minecraft Bedrock](https://img.shields.io/badge/Minecraft_Bedrock-1.26.51-62B47A)
[![LeviLamina](https://img.shields.io/badge/LeviLamina-26.51.6-4A90D9)](https://github.com/LiteLDev/LeviLamina)

</div>

A structure helper for Minecraft Bedrock. Import `.mcstructure` files, place a marker in the world, and build over the ghost blocks layer by layer, with a material list for the full build, fully client side in Levi Lamina, written in Rust for high performance using Bedrock's native rendering engine.

Stratum is everything Structura and Holoprint should be if they weren't restricted by Minecraft Bedrock's scripting engine -- a fully litematica replacement for Minecraft Bedrock edition

## Features

- Projects with structure libraries
- Import a .mcstructure while in the game from Window's native file picker. No more converting strucutres to structura or holoprint.
- Persistent markers that are locked to their position and settings in each world. Create markers for all of your projects, load them when you need them, and pick off _exactly_ where you left them. No more re-adjusting holoprints or structura packs to get them locked to the right position.
- Rotate, move, and adjust the opacity and layer for each structure independently, with ghost blocks for the active marker
- Easy Place - grab your items in your inventory then right click to auto-place and correctly orient blocks from your inventory
- Material lists with text export, item counts, and stacks with sorting and filtering
- In game rendering previews of every structure

## Installation

Requires the client build of [LeviLamina](https://github.com/LiteLDev/LeviLamina) and [Alaydriem's Mod Manager](https://github.com/alaydriem/levi-modmanager).

```powershell
lip install github.com/alaydriem/levi-stratum#client
```

Or download the zip from the [latest release](https://github.com/alaydriem/levi-stratum/releases/latest) and extract it into `mods/`.

| Stratum | Minecraft Bedrock | LeviLamina | Mod Manager |
| --- | --- | --- | --- |
| 0.1.0 | 1.26.51 | 26.51.6+ | 0.1.x |

## Usage

1. Press <kbd>Alt</kbd> + <kbd>S</kbd> to open the library.
2. Import a `.mcstructure` file, or drop files into `mods/alaydriem-stratum/data/import`. New files go to Unsorted.
3. Select a structure or a project and click **Place marker here**. The marker is placed at your feet and becomes the active marker.
4. Close the library. Use the HUD keys to move through the layers and adjust the marker.

## Controls

| Key | Action |
| --- | --- |
| <kbd>Alt</kbd> + <kbd>S</kbd> | Open / close the library |
| <kbd>F10</kbd> | Change the HUD between Quick and All controls |

The HUD keys work while no screen is open and a marker is active.

### Quick mode

| Key | Action |
| --- | --- |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Layer up / down |
| <kbd>←</kbd> | Show / hide the ghost |
| <kbd>→</kbd> | Change the opacity |

### All controls mode

<kbd>←</kbd> and <kbd>→</kbd> select a control. <kbd>↑</kbd> and <kbd>↓</kbd> change it.

| Control | Action |
| --- | --- |
| Layer | Layer up / down |
| Opacity | Opacity up / down |
| Show | Show / hide the ghost |
| Structure | Next / previous structure (only when the marker shows a project) |
| Move X / Y / Z | Move the marker one block along the axis |
| Rotate | Rotate the marker |

All keys can be changed on the mod manager's Keybinds page.

## Configuration

Settings are in the mod manager panel and saved to `config.hcl` in the mod's config folder.

<details>
<summary>All settings</summary>

| Section | Setting | Default | Description |
| --- | --- | --- | --- |
| `keys` | `open_library` | `"Alt+S"` | Open / close the library |
| | `hud_up` / `hud_down` | `"Up"` / `"Down"` | HUD up / down |
| | `hud_left` / `hud_right` | `"Left"` / `"Right"` | HUD left / right |
| | `hud_mode` | `"F10"` | Change the HUD mode |
| `library` | `max_structures` | `4096` | Most structures (`16`–`65536`). Imports past this are refused |
| | `max_projects` | `512` | Most projects (`1`–`4096`) |
| `markers` | `max_per_world` | `256` | Most markers per world (`1`–`4096`) |
| `import` | `max_file_mb` | `256` | Largest structure file in MB (`1`–`256`) |
| | `inbox_poll_seconds` | `2.0` | Time between checks of the import folder (`0.5`–`30.0`) |
| `cache` | `budget_mb` | `128` | Memory for structures read for material lists (`64`–`2048`) |
| `ghost` | `miniature` | `true` | Show the full build in miniature at its lowest corner; off shows a structure block there |
| | `draw_distance` | `0` | Draw distance in chunks (`0`–`64`); `0` uses the game's render distance |
| | `mesh_budget_mb` | `1024` | Graphics memory the ghost blocks can use (`64`–`8192`) |
| | `cells_per_frame` | `600` | Cells built per frame (`64`–`2000`); higher builds faster and costs more frame time |
| `debug` | `telemetry` | `false` | Log tick and frame cost, thread wake-ups and saves every 2 s |

The `library`, `markers`, `import`, `cache` and `ghost.mesh_budget_mb` settings apply after a restart.

</details>

## FAQ

**Can I use it on servers?** Yes. Stratum only draws on your screen and does not place blocks.

**Which file types can I import?** `.mcstructure` only, as saved by a structure block or a structure export tool.

**Why are some parts of a big build missing?** The ghost memory budget is full. Raise `ghost.mesh_budget_mb` or lower `ghost.draw_distance`.

## Building

Requires Windows x64, [mise](https://mise.jdx.dev), rustup, [cargo-nextest](https://nexte.st) and LLVM (`clang-cl`). The [`crates`](https://github.com/alaydriem/levi-crates) and [`modmanager`](https://github.com/alaydriem/levi-modmanager) repositories must be next to this one.

```powershell
mise trust
mise run preflight
mise run build-mod
mise run test
```

## License

BSD 3-Clause. See [LICENSE](./LICENSE). [Nordwand Mono](https://github.com/tywr/Nordwand-Mono) is under the SIL Open Font License 1.1.
