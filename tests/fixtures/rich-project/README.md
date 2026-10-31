# Mixed rich-project fixture

This fixture is a minimal, redistributable, local-only project used by
`scriberig-project` conformance tests. The source builders live in
`crates/scriberig-project/tests/rich_artifact_onboarding/support.rs`; the
ignored `generate_checked_in_rich_project_fixture` test regenerates its files
using the repository's locked Rust dependencies. The test copies each data file
to a fresh temporary directory and never edits the checked-in copy.

Semantic anchors:

- `source.rs`: Rust function `parcel_area` with a two-statement body, imported
  through the normal Rust source adapter in the same project snapshot as the
  rich artifacts.
- `budget.xlsx`: worksheet `Budget`, `A3 = 10`, `B3` formula `=A3*2` with
  cached result 20, `C3 = "existing"`, and workbook defined name
  `BudgetStart = Budget!$A$3`; worksheet `Notes`, `A1 = "preserve me"`.
- `report.docx`: first paragraph `Existing site area` and an opaque custom XML
  package member used to verify lossless preservation during semantic edits.
- `drawing.dxf`: layers `PARCELS` and `LABELS`, a three-vertex parcel polyline,
  the `Parcel 124` label, and an unrelated circle used to verify preservation.
- `project.qgs`: EPSG:4326 project CRS, a `Planning` layer group, categorized
  renderer configuration, and `Parcels`/`Roads` references to the GeoPackage
  and Shapefile fixtures. The `Roads` tree entry is initially hidden.
- `project.qgz`: bounded archive-form QGIS project with a `Parcels` reference.
- `parcels.gpkg`: `parcels` layer with `parcel_id`, `area`, and `name` fields;
  EPSG:4326; catalog extent `[1, 2, 3, 4]`; features `parcel_id=124` and
  `parcel_id=125` have distinct point geometries.
- `roads.shp/.shx/.dbf/.prj`: one local point feature with a numeric
  `road_id` field and explicit WGS 84 projection evidence.
- `unresolved.qgs`: retained QGIS reference to a missing local GeoPackage,
  expected to remain unresolved with a diagnostic.

No fixture accesses a network resource. All files are synthetic test data and
contain no credentials or third-party sample content. XLSX parser/edit behavior
uses the pinned `rxls` OOXML reader; the checked-in fixture is opened through the
project adapter onboarding test before edits are exercised. Recreate all binary
fixtures with `cargo test -p scriberig-project --test
rich_artifact_onboarding generate_checked_in_rich_project_fixture -- --ignored`.
