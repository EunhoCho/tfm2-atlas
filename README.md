# TFM2 Atlas 1.0.33

TFM2 Atlas is a pair of portable Teamfight Manager 2 `0.5.5` tools with a shared game-data foundation:

- **TFM2 Atlas Dashboard**: career statistics, champion tiers, champion information and manual Draft analysis.
- **TFM2 Atlas Editor**: player, staff, contract, transfer, economy, mastery and lock editing.

The Dashboard and Editor executables are separate applications with separate settings, single-instance locks and bridge ports. Dashboard does not load or launch Editor code.

## Source layout

```text
engine/              Shared calculation and protocol domain
atlas-core/          Dashboard Stable ABI service (127.0.0.1:28452)
atlas-client-055/    TFM2 0.5.5 active-catalog and client synchronization companion
atlas-editor/        Editor Stable ABI service (127.0.0.1:28453)
desktop/dashboard/   TFM2 Atlas Dashboard Electron application
desktop/editor/      TFM2 Atlas Editor Electron application
tools/               Build, release and naming-audit scripts
```

## Runtime combinations

- Dashboard: `tfm2_atlas_core` + `tfm2_atlas_client_055`
- Editor: `tfm2_atlas_core` + `tfm2_atlas_client_055` + `tfm2_atlas_editor`

The Dashboard executable can run without any Editor executable or Editor mod installed. The Editor intentionally uses Core for the active champion catalog and official positions.

## Development

Build and test the Rust crates with the official TFM2 `0.5.5` Stable ABI SDK. Each Electron project is independently testable and packageable from its own directory. Build outputs, SDK copies, `node_modules`, release candidates and game data are excluded from source control.

No Git stage, commit or push is performed by the build or release scripts.

## Licensing

Code and other material authored for Atlas are provided under the repository's
MIT license. That license does not replace licenses or rights attached to
upstream and third-party material.

- Dashboard provenance: [`desktop/dashboard/UPSTREAM_NOTICE.md`](desktop/dashboard/UPSTREAM_NOTICE.md)
- Editor provenance: [`desktop/editor/UPSTREAM_NOTICE.md`](desktop/editor/UPSTREAM_NOTICE.md)
- Repository license map: [`LICENSING.md`](LICENSING.md)
