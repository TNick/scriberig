# Rich Artifact Support

This matrix describes the capabilities implemented in the current workspace.
Capability is determined from exact artifact bytes and adapter fidelity; a file
extension alone does not make an artifact editable. Operations create semantic
candidates and use the shared validation and apply/materialization workflow.

## Capability Matrix

| Format | Inspect / import / query | Semantic edit | External reconciliation | Validate | Materialize |
|---|---|---|---|---|---|
| XLSX | Workbook, worksheet, cell, formula, style, and bounded range views. | Supported cell updates and sheet rename operations when the package passes the adapter's editability probe. | Read-only; no baseline-bound three-way reconciliation for external workbook edits. | Reopens the staged package and checks target semantics; apply also checks the exact validated-output digest. | Writes through the shared candidate path and records the new exact source digest. |
| DOCX | Document, section, paragraph, table, and cell paragraph semantics. | Safe paragraph replacement/insertion and supported table-cell paragraph updates; unsupported structures restrict editing. | Read-only; paragraph identity reconciliation for external package edits is not implemented. | Reopens staged DOCX, checks package structure and exact semantic target change, and rejects collateral semantic changes. | Shared materialization carries exact unchanged ZIP members and archive comments as opaque evidence. |
| DXF | Drawing, layers, supported entities, text search, bounded entity pages, and bounds queries. | Supported line geometry, text, and layer operations when the entire source maps to the supported subset. | Read-only; no baseline-bound DXF three-way merge. | Reopens staged DXF and verifies supported semantic completeness; apply checks the exact validated-output digest. | Shared candidate and filesystem materialization path; incomplete/unsupported drawings cannot be written. |
| DWG | Exact source artifact evidence is retained; semantic CAD inspection is unavailable without an injected provider. | Unavailable. No native provider is shipped or selected. | Unavailable. | Unavailable without a provider that advertises and performs reopen validation. | Unavailable; native DWG writes are never substituted with hidden DXF overwrites. |
| GeoPackage | Dataset/layer schemas, CRS, bounded feature pages, attribute filters, and spatial queries. | Supported feature geometry and attribute candidates for layers whose writer capability is proven. | Read-only; external package reconciliation is not implemented. | Reopens the staged GeoPackage and checks schema, CRS, target feature, and candidate digest. | Shared candidate/materialization path writes exact package bytes and updates source evidence. |
| Shapefile | Groups `.shp`, `.shx`, `.dbf`, and optional `.prj`; reads supported 2D dataset/layer/feature semantics. | Read-only. Multi-member output is not advertised because atomic Shapefile writing is not implemented. | Read-only. | Import checks member completeness and supported reader constraints; there is no write validation. | Not supported for edits. Complete groups are retained as source evidence; incomplete groups receive diagnostics. |
| QGS / QGZ | Project composition, layers, references, and supported renderer/configuration summaries. | Supported layer-reference rename candidate; referenced GeoPackage/Shapefile data remains a separate artifact. | Read-only; external QGIS project reconciliation is not implemented. | Reopens staged project configuration and validates supported references. | Shared candidate/materialization path for supported QGS/QGZ changes. |
| Unsupported formats | Exact scan/source bytes remain available as generic artifact or raw/binary evidence. | No semantic edit operation. | Not applicable. | No semantic validator is claimed. | No semantic materialization is claimed. |

“Read-only” external reconciliation means an externally edited file is not
silently merged into canonical semantic state by a format-specific three-way
reconciler. It does not prevent supported semantic operations from creating
reviewable candidates for formats whose edit column allows them.

## Fidelity Limits

- XLSX editability is limited to the package structures the current writer can
  safely round-trip. An unsupported or non-preservable workbook is read-only.
- DOCX editing is limited to structures the conservative writer can safely
  target. Untargeted package members are preserved exactly at the uncompressed
  member-payload level; output ZIP ordering and compression are not guaranteed.
- DXF write capability requires a complete mapping to the supported CAD model.
  Unsupported entities/records make the artifact read-only with a diagnostic;
  they are not claimed to be preserved by the DXF writer.
- GeoPackage edits are limited to layers/features supported by the staged
  writer. Source fidelity and output validation are tied to exact package
  digests.
- Shapefile support is read-only and currently limited to the implemented 2D
  reader. Missing sidecars, unsupported DBF memo data, row-count mismatch, or
  unsupported geometry must not be treated as writable semantic state.
- QGIS configuration semantics do not absorb or copy referenced dataset
  content. Unresolved/unsupported references remain explicit diagnostics.
- DWG has no installed/included provider. The provider-neutral CAD boundary is
  available for a future host-injected integration, but no real DWG import or
  write capability is claimed.
- Files without a supported semantic adapter remain inspectable as raw source
  evidence where policy allows, but cannot silently promote an unsafe write.

## Dependencies and Deployment

The current rich-artifact paths do not require Word, Excel, AutoCAD, QGIS, GDAL,
network access, or a native geospatial system library. DOCX/XLSX package handling
uses the Rust ZIP dependency; DXF uses the Rust `dxf` crate; GeoPackage uses the
workspace's bundled SQLite support; and Shapefile parsing is implemented in
Rust. The DWG provider boundary ships no SDK, native loader, binary, license
key, or provider credential. A real DWG integration still needs an explicit
licensing and deployment decision before it can be selected or bundled; see
[task 159](design/level-3/159.%20implement%20optional%20dwg%20provider%20boundary%20and%20licensing%20safe%20cad%20capability.md).

## Agent Operations

Agents should use semantic handles and operations. These operations create
ordinary candidates for user review and shared validation/apply; agents should
not write artifact bytes directly.

- Spreadsheet: `spreadsheet.read_range`, `spreadsheet.set_cell_value`,
  `spreadsheet.rename_sheet`.
- Rich document: `document.replace_paragraph`, `document.insert_paragraph`.
- CAD: `cad.update_line_geometry`, `cad.change_entity_layer`.
- Geospatial: `geo.set_attribute`, `geo.replace_geometry`.
- QGIS project: `qgis.rename_layer_reference` after resolving the reference to
  its semantic handle.
- Before editing, use `artifact.inspect` and the domain's exact-snapshot
  inspection/query operations to confirm fidelity, target identity, and
  snapshot context.
