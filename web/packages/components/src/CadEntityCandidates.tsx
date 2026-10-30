/** @file Renders safe CAD entity candidate forms and semantic change summaries. */
import { useEffect, useState, type ReactElement } from "react";
import type { CadEntitySummary, CadLayerSummary, CadLineEditResponse } from "@scriberig/protocol";
import type { CadPageController } from "@scriberig/model";

/** Renders one entity and the candidate operations supported by the shared model.
 * @param props Exact semantic entity, layer choices, and operation controller.
 * @returns Geometry detail and fidelity-gated candidate forms.
 */
export function CadEntityCandidates({
  controller,
  entity,
  layers,
  editable,
  onCandidateCreated,
}: {
  /** Exact-snapshot model controller. */ controller: CadPageController;
  /** Semantic entity summary. */ entity: CadEntitySummary;
  /** Layers in the exact drawing snapshot. */ layers: CadLayerSummary[];
  /** Whether the imported drawing advertises complete supported semantics. */ editable: boolean;
  /** Publishes a durable candidate and semantic before/after description. */
  onCandidateCreated: (candidate: CadLineEditResponse, summary: string) => void;
}): ReactElement {
  const [error, setError] = useState<string>();
  return (
    <article data-entity-id={entity.id} data-layer-id={entity.layer}>
      <h3>{geometryName(entity.geometry)}</h3>
      <pre>{JSON.stringify(entity.geometry, null, 2)}</pre>
      {isLine(entity.geometry) && editable ? (
        <LineCandidateForm
          controller={controller}
          entity={entity.id}
          geometry={entity.geometry.Line}
          onCandidate={(candidate, summary) => onCandidateCreated(candidate, summary)}
          onError={setError}
        />
      ) : null}
      {isLine(entity.geometry) && !editable ? (
        <p role="status">This drawing is read-only because source fidelity is incomplete.</p>
      ) : null}
      {editable && layers.some((layer) => layer.id !== entity.layer && !layer.locked) ? (
        <LayerCandidateForm
          controller={controller}
          entity={entity.id}
          currentLayer={entity.layer}
          layers={layers}
          onCandidate={(candidate, destination) =>
            onCandidateCreated(
              candidate,
              `CAD entity ${entity.id} moves from layer ${layerName(layers, entity.layer)} to ${destination}.`,
            )
          }
          onError={setError}
        />
      ) : null}
      {error ? <p role="alert">{error}</p> : null}
    </article>
  );
}

/** Creates a reviewable candidate that moves an entity to another unlocked layer.
 * @param props Exact target, layer choices, and candidate callbacks.
 * @returns Layer selector and candidate action.
 */
function LayerCandidateForm({
  controller,
  entity,
  currentLayer,
  layers,
  onCandidate,
  onError,
}: {
  /** Exact-snapshot model controller. */ controller: CadPageController;
  /** Semantic entity identity. */ entity: string;
  /** Current semantic layer identity. */ currentLayer: string;
  /** Layers in the drawing snapshot. */ layers: CadLayerSummary[];
  /** Candidate and destination display-name callback. */
  onCandidate: (candidate: CadLineEditResponse, destination: string) => void;
  /** Safe failure callback. */ onError: (message: string) => void;
}): ReactElement {
  const choices = layers.filter((layer) => layer.id !== currentLayer && !layer.locked);
  const choiceIdentity = choices.map((layer) => layer.id).join("\u0000");
  const [destination, setDestination] = useState(choices[0]?.id ?? "");
  useEffect(() => {
    setDestination(choices[0]?.id ?? "");
  }, [choiceIdentity]);
  /** Creates a shared layer-change candidate for the selected destination.
   * @returns Promise that settles after publishing the candidate or safe error.
   */
  async function propose(): Promise<void> {
    onError("");
    try {
      const candidate = await controller.changeLayer(entity, destination);
      onCandidate(candidate, layerName(layers, destination));
    } catch (reason) {
      onError(String(reason));
    }
  }
  return (
    <fieldset>
      <legend>Propose layer change</legend>
      <label>
        Destination layer
        <select value={destination} onChange={(event) => setDestination(event.target.value)}>
          {choices.map((layer) => (
            <option key={layer.id} value={layer.id}>
              {layer.name}
            </option>
          ))}
        </select>
      </label>
      <button type="button" onClick={() => void propose()} disabled={destination === ""}>
        Create candidate
      </button>
    </fieldset>
  );
}

/** Creates a reviewable line candidate and describes changed endpoints.
 * @param props Target geometry and candidate callback.
 * @returns Controlled exact-decimal coordinate fields and candidate action.
 */
function LineCandidateForm({
  controller,
  entity,
  geometry,
  onCandidate,
  onError,
}: {
  /** Exact-snapshot model controller. */ controller: CadPageController;
  /** Semantic line identity. */ entity: string;
  /** Existing line coordinates. */ geometry: {
    start: { x: string; y: string; z: string };
    end: { x: string; y: string; z: string };
  };
  /** Candidate and semantic summary callback. */
  onCandidate: (candidate: CadLineEditResponse, summary: string) => void;
  /** Safe failure callback. */ onError: (message: string) => void;
}): ReactElement {
  const [start, setStart] = useState(geometry.start);
  const [end, setEnd] = useState(geometry.end);
  /** Creates a shared candidate for the edited line endpoints.
   * @returns Promise that settles after publishing the candidate or safe error.
   */
  async function propose(): Promise<void> {
    onError("");
    try {
      const candidate = await controller.replaceLine(entity, start, end);
      onCandidate(
        candidate,
        `CAD line ${entity} endpoints change from ${coordinate(geometry.start)} → ${coordinate(geometry.end)} to ${coordinate(start)} → ${coordinate(end)}.`,
      );
    } catch (reason) {
      onError(String(reason));
    }
  }
  return (
    <fieldset>
      <legend>Propose line endpoints</legend>
      <CoordinateFields label="Start" value={start} onChange={setStart} />
      <CoordinateFields label="End" value={end} onChange={setEnd} />
      <button type="button" onClick={() => void propose()}>
        Create candidate
      </button>
    </fieldset>
  );
}

/** Renders three exact decimal coordinate inputs.
 * @param props Coordinate values and state updater.
 * @returns Accessible x, y, and z inputs.
 */
function CoordinateFields({
  label,
  value,
  onChange,
}: {
  /** Coordinate group label. */ label: string;
  /** Current exact decimal ordinates. */ value: { x: string; y: string; z: string };
  /** Updates one coordinate group. */
  onChange: (value: { x: string; y: string; z: string }) => void;
}): ReactElement {
  return (
    <div>
      {(["x", "y", "z"] as const).map((axis) => (
        <label key={axis}>
          {label} {axis.toUpperCase()}
          <input
            value={value[axis]}
            onChange={(event) => onChange({ ...value, [axis]: event.target.value })}
          />
        </label>
      ))}
    </div>
  );
}

/** Checks whether typed geometry is a line variant.
 * @param geometry Semantic geometry variant.
 * @returns Whether a line edit operation is available.
 */
function isLine(
  geometry: CadEntitySummary["geometry"],
): geometry is Extract<CadEntitySummary["geometry"], { Line: unknown }> {
  return "Line" in geometry;
}

/** Formats one exact coordinate without numeric conversion.
 * @param value Exact decimal coordinate.
 * @returns Parenthesized x, y, and z values.
 */
function coordinate(value: { x: string; y: string; z: string }): string {
  return `(${value.x}, ${value.y}, ${value.z})`;
}

/** Resolves an authored layer name for a semantic layer identity.
 * @param layers Exact drawing layers.
 * @param id Semantic layer identity.
 * @returns Authored name or the stable ID when the layer is absent.
 */
function layerName(layers: readonly CadLayerSummary[], id: string): string {
  return layers.find((layer) => layer.id === id)?.name ?? id;
}

/** Returns a concise name for the semantic geometry variant.
 * @param geometry Semantic CAD geometry value.
 * @returns Stable geometry variant name.
 */
function geometryName(geometry: CadEntitySummary["geometry"]): string {
  return Object.keys(geometry)[0] ?? "Unknown geometry";
}
