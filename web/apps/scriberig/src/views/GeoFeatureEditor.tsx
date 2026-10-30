/** @file Renders capability-gated semantic edits for a selected GeoPackage feature. */

import { Box, Button, MenuItem, Stack, TextField, Typography } from "@mui/material";
import type { ReactElement } from "react";
import { summarizeGeoAttributeChange, summarizeGeoGeometryChange } from "@scriberig/components";
import type { GeoFeatureView, GeoGeometry, GeoLayerView, GeoValue } from "@scriberig/protocol";
import type { GeoPageController } from "@scriberig/model";

/** Renders schema, CRS, extent, and fidelity diagnostics for one layer.
 * @param props Exact semantic layer summary.
 * @returns Readable layer metadata and edit capability status.
 */
export function LayerSummary({ layer }: { layer: GeoLayerView }): ReactElement {
  return (
    <Box component="section" aria-label="Layer schema" sx={{ my: 2 }}>
      <Typography>
        {crsLabel(layer)} · {layer.geometryType ?? "unknown geometry"} · {layer.featureCount} source
        features
      </Typography>
      <Typography variant="body2">
        Fields:{" "}
        {layer.fields.map((field) => `${field.name} (${field.fieldType})`).join(", ") || "none"}
      </Typography>
      <Typography variant="body2">
        {layer.editable ? "Editable with semantic candidates" : "Read only"}
      </Typography>
      {layer.diagnostics.map((diagnostic) => (
        <Typography key={diagnostic} role="note">
          {diagnostic}
        </Typography>
      ))}
    </Box>
  );
}

/** Renders typed attribute and geometry candidate controls for one feature.
 * @param props Exact loaded layer, feature, and editor callbacks.
 * @returns Candidate-only attribute and geometry edit forms.
 */
export function FeatureEditor({
  controller,
  layer,
  feature,
  crsLabel,
  field,
  value,
  geometryText,
  activeField,
  onField,
  onValue,
  onGeometry,
  onCandidate,
  onError,
}: {
  /** Application controller for proposing semantic changes. */
  controller: GeoPageController | undefined;
  /** Exact selected semantic layer and editability declaration. */
  layer: GeoLayerView;
  /** Exact selected semantic feature. */
  feature: GeoFeatureView;
  /** Coordinate reference system used by replacement geometry ordinates. */
  crsLabel: string;
  /** Selected field name. */
  field: string;
  /** Authored replacement attribute text. */
  value: string;
  /** Authored replacement geometry JSON. */
  geometryText: string;
  /** Selected field schema when available. */
  activeField: GeoLayerView["fields"][number] | undefined;
  /** Updates the selected attribute name. */
  onField: (field: string) => void;
  /** Updates replacement attribute text. */
  onValue: (value: string) => void;
  /** Updates replacement geometry text. */
  onGeometry: (value: string) => void;
  /** Reports a created shared candidate identity. */
  onCandidate: (candidate: string, summary: string) => void;
  /** Reports validation or proposal failure. */
  onError: (error: string) => void;
}): ReactElement {
  return (
    <Stack
      component="section"
      aria-label={`Feature ${feature.id}`}
      spacing={1}
      sx={{ maxWidth: 720, my: 2 }}
    >
      <Typography variant="h6">Feature {feature.nativeId ?? feature.id}</Typography>
      <Typography variant="body2">Semantic ID: {feature.id}</Typography>
      <Box component="dl" aria-label="Feature attributes">
        {Object.entries(feature.attributes).map(([name, item]) => (
          <Box key={name} component="div">
            <Typography component="dt">{name}</Typography>
            <Typography component="dd">
              {item.kind === "null" ? "null" : String(item.value)}
            </Typography>
          </Box>
        ))}
      </Box>
      <Stack direction="row" spacing={1}>
        <TextField
          select
          label="Attribute"
          value={field}
          disabled={!layer.editable}
          onChange={(event) => onField(event.target.value)}
        >
          {layer.fields
            .filter((item) => item.fieldType !== "binary")
            .map((item) => (
              <MenuItem key={item.name} value={item.name}>
                {item.name} · {item.fieldType}
              </MenuItem>
            ))}
        </TextField>
        <TextField
          label="New value"
          value={value}
          disabled={!layer.editable}
          onChange={(event) => onValue(event.target.value)}
        />
        <Button
          disabled={!layer.editable || activeField === undefined}
          onClick={() => {
            try {
              const typed = typedValue(activeField, value);
              void controller
                ?.proposeAttribute(layer.id, feature.id, field, typed)
                .then((candidate) =>
                  onCandidate(
                    candidate.candidate,
                    summarizeGeoAttributeChange(feature, field, typed),
                  ),
                )
                .catch((reason: unknown) => onError(String(reason)));
            } catch (reason) {
              onError(String(reason));
            }
          }}
        >
          Propose attribute
        </Button>
      </Stack>
      <TextField
        label={`Replacement geometry JSON (${crsLabel})`}
        multiline
        minRows={4}
        value={geometryText}
        disabled={!layer.editable}
        onChange={(event) => onGeometry(event.target.value)}
        helperText={`Coordinates are interpreted directly in ${crsLabel}.`}
      />
      <Button
        disabled={!layer.editable}
        onClick={() => {
          try {
            const geometry = JSON.parse(geometryText) as GeoGeometry;
            void controller
              ?.proposeGeometry(layer.id, feature.id, geometry)
              .then((candidate) =>
                onCandidate(candidate.candidate, summarizeGeoGeometryChange(feature, geometry)),
              )
              .catch((reason: unknown) => onError(String(reason)));
          } catch (reason) {
            onError(String(reason));
          }
        }}
      >
        Propose geometry
      </Button>
    </Stack>
  );
}

/** Parses editor text into a schema-matched typed GeoValue.
 * @param field Exact field schema.
 * @param value Authored editor text.
 * @returns Typed geospatial attribute value.
 * @throws When the text cannot represent the selected scalar type.
 */
function typedValue(field: GeoLayerView["fields"][number] | undefined, value: string): GeoValue {
  if (field === undefined) throw new TypeError("select an editable attribute first");
  if (field.fieldType === "text") return { kind: "text", value };
  if (field.fieldType === "number") {
    if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(value)) {
      throw new TypeError("number field requires decimal text");
    }
    return { kind: "number", value };
  }
  if (field.fieldType === "boolean") {
    if (value !== "true" && value !== "false") throw new TypeError("boolean must be true or false");
    return { kind: "boolean", value: value === "true" };
  }
  if (field.fieldType === "date_time") return { kind: "date_time", value };
  throw new TypeError("binary fields are read only");
}

/** Formats exact CRS metadata without implying a coordinate transformation.
 * @param layer Layer supplying authored coordinate metadata.
 * @returns Authority/code, explicit definition, or an unavailable label.
 */
export function crsLabel(layer: GeoLayerView): string {
  if (layer.crs === undefined) return "CRS unavailable";
  if (layer.crs.authority && layer.crs.code) return `${layer.crs.authority}:${layer.crs.code}`;
  return layer.crs.definition ?? "CRS unavailable";
}
