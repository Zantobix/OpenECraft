# Installation plan pages – every change of the fork, item by item

Branch `feature/installation-plan-page` of `Zantobix/OpenECraft` (issue nilres/OpenECraft#277), plus the separate fix
branch `fix/undo-stale-analysis`. Each line is one change a reviewer can look for; the commit is given in brackets. For the pull request all of it is
squashed into one commit (`04a825c` on top of the current master); the single commits are kept on
`backup/installation-plan-page-2`.
Kept up to date with every round of work (see the private notes, `CLAUDE.md`).

## 1. New page type "Installation plan"

- New page kind `InstallationPlan(level)`: one level of a building as a printable page (PDF, SVG, DXF, print) like
  every other page, drawn through `render::Draw` (`render/installation_plan.rs`). [4091ef9]
- *New installation plan page* button in *Buildings › Electrical installation*; the page type in *Page properties*;
  `add_page kind=installation_plan building=… level=…` on the command line / MCP. [4091ef9]
- The page type list has **one** entry "Installation plan"; the **building level** is chosen in its own field
  (*Building level*), placed right above the rooms; choosing another level clears the rooms chosen. [57cd379, 286e04c]
- Page icon in the page tree, title block page type, connector key, `ops::add_page` description. [4091ef9]
- Existing projects are unchanged: all new data is serialized only when it differs from the default. [4091ef9]

## 2. What the page draws

- Walls of the rooms as filled bands with **gaps for doors and windows**, at least 0.5 mm thick on paper. [4091ef9]
- **Door leaves and swing arcs**, window panes. [4091ef9]
- **Room names**, moved out of the way when a ceiling light sits in the middle of the room. [4091ef9]
- **Room descriptions** introduced: below the location identifier (`+KU`) the description of that identifier from the
  structure management (`Küche`); a *Description* field in the room's properties edits it (stored once, in the
  structure management, `define_structure`). [81e4454]
- Every installation device with its **plan symbol turned towards its wall** (connection point at the wall); ceiling
  devices centred. [4091ef9, 8c98094]
- **Device tags** beside the symbols, placed on the room side away from the wall. [4091ef9]
- **Connections of the circuits** from device to device, dashed, along the walls (x first, then y). [4091ef9]
- **Circuit colours** for symbols and connections, shared with the site plan on screen (one colour table). [4091ef9]
- **Legend of the circuits** (protective device and its function text in its colour). [4091ef9]
- **Legend texts are wrapped** at blanks inside their column, so that long function texts stay inside the frame
  instead of running over it; a legend row is as tall as its longest entry; the width is taken as printed
  (Helvetica metrics), not by the narrower canvas estimate. [9dc1735, 906cf91]
- **Cabinets and distribution boards** standing in a room drawn with their footprint as the distribution board symbol
  (IEC 60617-11, half filled) and their name. [0bf6774]
- **Column and row strips** of the plot frame (1, 2 … / A, B …) on installation plan pages too. [daa1f8d]
- **Caption** "building level – Scale 1:n", with "(fitted)" when the scale was fitted. [4091ef9, daa1f8d]
- **PDF links**: a symbol and a legend line link to the installation page of its circuit. [4091ef9]
- A missing level is reported on the page ("Level not found – choose another one in the page properties"). [4091ef9]

## 3. View options per page (page properties and `set_page`)

- Device tags on/off (`tags`). [4091ef9]
- Circuit colours on/off (`circuit_colors`). [4091ef9]
- Connections on/off (`connections`). [4091ef9]
- Room identifiers on/off (`room_names`) and room descriptions on/off (`room_descriptions`), each on its own. [4091ef9, 81e4454]
- **Tag parts**: tags shortened to some of their IEC 81346 aspects (`tag_parts`, e.g. only `-X1.6` of `=SD1+F1-X1.6`).
  [4091ef9]
- *Tags shown as* with plain choices and an **example taken from the project**: full tag, location and device,
  function and device, device only, or custom (the four check boxes only under *Custom*), replacing four bare check
  boxes that users found confusing. [daa1f8d]
- **Legend on/off** and its **position**: right, left (a column) or top, bottom (a band with several columns)
  (`legend`, `legend_at`). [81e4454]
- **Font sizes in pt** (as printed) per page for tags (and cabinet names), rooms and legend (`tag_size`,
  `room_size`, `legend_size`); first added as text heights in mm, changed to pt on user request. [81e4454, 163dfee]
- **Symbol sizes** in % per page: the device symbols and the cabinet symbols, the cabinet scaled about its middle
  (`symbol_scale`, `cabinet_scale`). [163dfee]
- **Rooms of the page** (detail): tick some rooms of the level (`set_page rooms=+BATH,+KIT`); the page is centred on
  or fitted to them, the legend lists only their circuits. [daa1f8d]

## 4. Scale and fitting

- A fixed scale 1:10 … 1:500 is kept (or typed); enlarging scales (x:1), which made no sense for a building, are no
  longer offered for these pages. [daa1f8d]
- *Fit onto the page* is a choice of its own and **stays chosen**: the page is fitted again whenever rooms, devices
  or the level change. [daa1f8d, 286e04c]
- Fitting chooses from **fine usual scales** (1:12.5, 1:15, 1:20, 1:25, 1:30, 1:40, 1:50, 1:60, 1:75, 1:100 …) so
  that a room or a level fills the page, instead of jumping from 1:25 to 1:50. [57cd379]
- Fix: choosing *Fit onto the page* turned into 1:2, because the field for typing a scale was still drawn in the same
  frame and clamped the 1:1 that means fitting; the field is now asked after the choice. [286e04c]
- What does not fit into the drawing area is **cut off at its edge** (lines, outlines, filled areas, circles, texts,
  PDF links) by a clipping draw adapter, so a fixed scale or a chosen room shows a clean detail. [daa1f8d]

## 5. Plan symbols from the symbol library

- The site plan and the page no longer draw nine hard-coded shapes but **symbols of the library** (IEC 60617-11),
  scaled to plan size by a drawing adapter. [8c98094]
- **23 new installation symbols**: single, triple, switched, splash-proof and CEE socket outlets; intermediate,
  series and blind switches; wall, recessed, spot and emergency lights; device connection, data and antenna
  outlets; heating element, room thermostat, motor, fan, door intercom, bell, smoke alarm, window/door contact.
  [8c98094]
- A **part can bring its own plan symbol** (*Plan symbol* in the part's mounting data, `Part::plan_symbol`,
  `PLAN_SYMBOLS`); without one it gets the symbol of its kind and subgroup (two-way, intermediate and blind switches,
  dimmers, double socket outlets, wall and recessed lights …). [8c98094]
- Commands `plan_symbols` (list) and `set_plan_symbol part=… symbol=…`. [8c98094]
- The installation heating element symbol is named apart from the schematic one ("Heating element (installation)").
  [5f69003]

## 6. Command line / MCP

- `add_page kind=installation_plan building= level=`; `set_page` with `tags`, `circuit_colors`, `connections`,
  `room_names`, `room_descriptions`, `tag_parts`, `rooms`, `legend`, `legend_at`, `tag_size`, `room_size`,
  `legend_size`; `page_json` reports `level` and all of `plan`. [4091ef9 … 81e4454]
- Unknown values are refused with a message (unknown room, unknown legend position, text height out of range). [daa1f8d, 81e4454]

## 7. Texts, documentation, tests

- All new texts in English, German, French and Spanish (`required.tsv`, sorted by `tidy`). [every commit]
- `docs/features/panel-layout.md` and `docs/features/symbols.md` with their de, fr, es translations; picture
  `docs/images/installation-plan.png`; changelog entries in 0.1.4. [every commit]
- Tests in `render/installation_plan/tests.rs`: walls, room names, tags, caption, view options, missing level, scale,
  tag parts and presets, every plan symbol drawn, a part's own plan symbol and the fallback, defaults by subgroup,
  symbols towards their wall, serde of `PlanView`, cabinets, chosen rooms fitted, cut-off at the edge, frame strips,
  fine fitted scales, legend off and in all four positions, room descriptions and text heights; API tests of the new
  `set_page` options. [every commit]

## 8. Separate fix (own branch, not part of the plan pull request)

- **Crash on Ctrl+Z** after putting a device into a room in *Buildings › Electrical installation*: the pane read the
  circuits from the netlist of the state before (index out of bounds in `electrical::mains`). *Undo* and *Redo*
  (also several steps from the undo list) now analyse the project again at once; regression test that fails without
  the fix. Bug of the existing code, found while testing. [`fix/undo-stale-analysis` 1575b65]

## 9. Known limits (follow-ups)

- A door between two rooms has to be set in both rooms (each room has its own walls).
- Tag aspects are recognised in IEC order only (`=…++…+…-…`).
- Overlapping tags of neighbouring devices are not moved apart yet.
- Placing devices directly on the plan page (instead of the site plan of the Buildings pane) and a workspace of its own
  for buildings would change the existing workflow – to be agreed with the maintainer first.
