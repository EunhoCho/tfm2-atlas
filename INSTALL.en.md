# TFM2 Atlas 1.0.33 installation

## Dashboard only

1. Copy `Dashboard/mods/tfm2_atlas_core` and `Dashboard/mods/tfm2_atlas_client_055` into the game's lowercase `mods` directory.
2. Enable both code mods in game and start a career.
3. Run `TFM2.Atlas.Dashboard.exe` from any directory outside the game. If the release includes adjacent DLLs, keep them next to the EXE.

Dashboard retains all Dashboard features and tier application without the Editor executable or `tfm2_atlas_editor`.

## Add Editor

1. Add `Editor/mods/tfm2_atlas_editor` to the two common mods above.
2. The common mods included in both product folders are byte-identical; keep only one copy of each in the game's `mods` directory.
3. Run `TFM2.Atlas.Editor.exe` from any directory outside the game.

Editor reports Core and Editor connections separately. If any required mod is missing, it shows installation guidance instead of the editing forms.

Move old bridge or tier mods that use the same ports to a backup directory outside the game rather than deleting them. Never place the EXEs or application folders inside `mods`.
