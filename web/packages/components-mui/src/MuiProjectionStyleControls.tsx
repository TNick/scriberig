/** @file Renders shared projection-style schema, provenance, and preference controls. */

import { Button, Stack, TextField, Typography } from "@mui/material";
import type {
  ProjectionStyleResolveResponse,
  ProjectionStyleSchemaResponse,
} from "@scriberig/protocol";
import type { ReactElement } from "react";

/** Presents adapter-owned style metadata and forwards personal preference intent.
 * @param props Shared schema/resolution data and host-owned mutation callbacks.
 * @returns Domain-neutral style controls for one resolved projection.
 */
export function MuiProjectionStyleControls({
  schema,
  resolution,
  preference,
  busy = false,
  onPreferenceChange,
  onSave,
  onClear,
}: {
  /** Schema declared and validated by the domain adapter. */
  schema: ProjectionStyleSchemaResponse;
  /** Exact shared resolution, provenance chain, and fidelity. */
  resolution: ProjectionStyleResolveResponse;
  /** Current user preference in the adapter's UTF-8 input format. */
  preference: string;
  /** Whether a shared preference request is in progress. */
  busy?: boolean;
  /** Receives raw adapter-owned style input without parsing it in this package. */
  onPreferenceChange: (value: string) => void;
  /** Requests validation and persistence by the shared application service. */
  onSave: () => void;
  /** Requests clearing the current user's preference. */
  onClear: () => void;
}): ReactElement {
  return (
    <section aria-label="Projection style controls">
      <Typography variant="h6">Projection style</Typography>
      <Typography>
        {schema.reference.adapterId} · schema {schema.reference.version}
      </Typography>
      <Typography role="status">
        Effective fidelity: {resolution.fidelity.fidelity}
        {resolution.fidelity.omittedCategories.length > 0
          ? ` · omitted: ${resolution.fidelity.omittedCategories.join(", ")}`
          : ""}
      </Typography>
      {resolution.fidelity.explanation ? (
        <Typography>{resolution.fidelity.explanation}</Typography>
      ) : null}
      <ul aria-label="Effective style sources">
        {resolution.applied.map((source, index) => (
          <li key={`${source.kind}-${source.identity}-${index}`}>
            {source.kind}: {source.identity}
          </li>
        ))}
      </ul>
      <ul aria-label="Adapter style properties">
        {schema.properties.map((property) => (
          <li key={property.key}>
            {property.label} ({property.kind})
            {property.description ? `: ${property.description}` : ""}
            {property.allowedValues.length > 0
              ? ` · allowed: ${property.allowedValues.join(", ")}`
              : ""}
            {property.minimum !== undefined || property.maximum !== undefined
              ? ` · range: ${property.minimum ?? "−∞"} to ${property.maximum ?? "∞"}`
              : ""}
          </li>
        ))}
      </ul>
      <Stack spacing={1}>
        <TextField
          label="Personal style input"
          multiline
          minRows={2}
          value={preference}
          onChange={(event) => onPreferenceChange(event.target.value)}
          helperText="The domain adapter validates and canonicalizes this UTF-8 value."
        />
        <Stack direction="row" spacing={1}>
          <Button disabled={busy || preference.length === 0} onClick={onSave}>
            Save personal style
          </Button>
          <Button disabled={busy} onClick={onClear}>
            Clear personal style
          </Button>
        </Stack>
      </Stack>
    </section>
  );
}
