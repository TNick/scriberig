/** @file Renders CAD entities and candidate-only semantic edit forms. */
import { useEffect, useState, type ReactElement } from "react";
import type {
  CadCoordinate,
  CadEntitySummary,
  CadLayerSummary,
  CadLineEditResponse,
} from "@scriberig/protocol";
import type { CadPageController } from "@scriberig/model";
import { summarizeCadLayerChange, summarizeCadLineChange } from "./semantic-change-summary";

/** Renders one semantic CAD entity and its supported candidate operation.
 * @param props Controller and one exact semantic entity.
 * @returns Geometry summary and supported candidate forms.
 */
export function CadEntityView({
  controller,
  entity,
  layers,
  editable,
  onCandidateCreated,
}: {
  /** Exact-snapshot model controller. */
  controller: CadPageController;
  /** Semantic entity summary. */
  entity: CadEntitySummary;
  /** Layers in the exact drawing snapshot. */
  layers: CadLayerSummary[];
  /** Whether the imported drawing advertises complete supported semantics. */
  editable: boolean;
  /** Publishes a durable candidate returned by the shared operation. */
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
          onCandidate={(candidate, previous, start, end) =>
            onCandidateCreated(
              candidate,
              summarizeCadLineChange(entity.id, previous, { start, end }),
            )
          }
          onError={setError}
        />
      ) : null}
      {editable && layers.some((layer) => layer.id !== entity.layer && !layer.locked) ? (
        <LayerCandidateForm
          controller={controller}
          entity={entity.id}
          currentLayer={entity.layer}
          layers={layers}
          onCandidate={(candidate, destination) => {
            const previousLayer = layers.find((layer) => layer.id === entity.layer)?.name;
            const nextLayer = layers.find((layer) => layer.id === destination)?.name;
            onCandidateCreated(
              candidate,
              summarizeCadLayerChange(
                entity.id,
                previousLayer ?? entity.layer,
                nextLayer ?? destination,
              ),
            );
          }}
          onError={setError}
        />
      ) : null}
      {isLine(entity.geometry) && !editable ? (
        <p role="status">This drawing is read-only because source fidelity is incomplete.</p>
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
  /** Candidate result callback with the exact proposed destination. */
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
      onCandidate(await controller.changeLayer(entity, destination), destination);
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

/** Creates a reviewable line geometry candidate from exact decimal fields.
 * @param props Target and operation callback.
 * @returns Controlled coordinate fields and candidate action.
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
    start: CadCoordinate;
    end: CadCoordinate;
  };
  /** Candidate result callback with exact proposed endpoints. */
  onCandidate: (
    candidate: CadLineEditResponse,
    previous: {
      start: CadCoordinate;
      end: CadCoordinate;
    },
    start: CadCoordinate,
    end: CadCoordinate,
  ) => void;
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
      onCandidate(await controller.replaceLine(entity, start, end), geometry, start, end);
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
  /** Current exact decimal ordinates. */ value: CadCoordinate;
  /** Updates one coordinate group. */ onChange: (value: CadCoordinate) => void;
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

/** Returns a concise name for the semantic geometry variant.
 * @param geometry Semantic geometry value.
 * @returns Stable geometry variant name.
 */
function geometryName(geometry: CadEntitySummary["geometry"]): string {
  return Object.keys(geometry)[0] ?? "Unknown geometry";
}
