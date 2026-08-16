# TFM2 Atlas licensing map

The root [`LICENSE`](LICENSE) is the MIT license for code and other material
authored by Eunho Cho for TFM2 Atlas. It does not relicense material owned by
other authors.

## Dashboard

Dashboard-specific provenance and attribution are documented in
[`desktop/dashboard/UPSTREAM_NOTICE.md`](desktop/dashboard/UPSTREAM_NOTICE.md).
The original reference distribution inspected during development did not
include a license from its dashboard authors. Its `DashboardApp/LICENSE` file
is Electron's MIT license and is not a license for the original dashboard
implementation.

## Editor

Atlas Editor's current Stable ABI service and Electron interface are
independent implementations and do not include the referenced Rust/egui
application, legacy bridge, binaries, documentation, artwork or other upstream
implementation material. Product-specific provenance is documented in
[`desktop/editor/UPSTREAM_NOTICE.md`](desktop/editor/UPSTREAM_NOTICE.md).

The referenced project's source-available license is therefore not presented
as a license for Atlas and is not bundled. If upstream implementation material
is introduced in the future, its applicable license and notices must be
restored before release.

## Bundled dependencies

Electron, Chromium, Noto Sans KR, Rust crates, JavaScript packages and other
third-party components remain under their own licenses. Product release
folders include the corresponding notices and generated license files.
