/** @file Normalizes CAD candidate responses for web consumers. */
import type {
  CadDrawingSpace,
  CadDrawingInspectResponse,
  CadEntitiesResponse,
  CadBoundsQueryResponse,
  CadGeometry,
  CadEntityStyleRef,
  CadLineEditResponse,
} from "@scriberig/protocol";

/** Normalizes a Rust CAD drawing summary response.
 * @param value Raw transport response.
 * @returns Snapshot-bound drawing metadata and layers.
 * @throws When required drawing fields or layer values are malformed.
 */
export function normalizeCadDrawingInspectResponse(value: unknown): CadDrawingInspectResponse {
  const response = record(value, "CAD drawing");
  if (!Array.isArray(response.layers) || !Array.isArray(response.diagnostics)) {
    throw new TypeError("CAD layers and diagnostics must be arrays");
  }
  return {
    project: string(response.project, "project"),
    snapshot: string(response.snapshot, "snapshot"),
    drawing: string(response.drawing, "drawing"),
    units: string(response.units, "units"),
    ...(response.coordinate_reference == null
      ? {}
      : { coordinateReference: string(response.coordinate_reference, "coordinate reference") }),
    layers: response.layers.map((value) => {
      const layer = record(value, "CAD layer");
      if (typeof layer.visible !== "boolean" || typeof layer.locked !== "boolean") {
        throw new TypeError("CAD layer flags must be boolean");
      }
      return {
        id: string(layer.id, "layer id"),
        name: string(layer.name, "layer name"),
        visible: layer.visible,
        locked: layer.locked,
      };
    }),
    entityCount: number(response.entity_count, "entity count"),
    complete: boolean(response.complete, "complete"),
    diagnostics: response.diagnostics.map((item) => string(item, "diagnostic")),
  };
}

/** Normalizes a Rust bounded CAD entity response.
 * @param value Raw transport response.
 * @returns Typed entity page with continuation metadata.
 * @throws When required page metadata or entities are malformed.
 */
export function normalizeCadEntitiesResponse(value: unknown): CadEntitiesResponse {
  const response = record(value, "CAD entities");
  if (!Array.isArray(response.entities)) throw new TypeError("CAD entities must be an array");
  return {
    project: string(response.project, "project"),
    snapshot: string(response.snapshot, "snapshot"),
    drawing: string(response.drawing, "drawing"),
    entities: response.entities.map((value) => {
      const entity = record(value, "CAD entity");
      return {
        id: string(entity.id, "entity id"),
        layer: string(entity.layer, "layer id"),
        space: normalizeSpace(entity.space),
        geometry: normalizeGeometry(entity.geometry),
        ...(entity.style == null ? {} : { style: normalizeStyle(entity.style) }),
      };
    }),
    total: number(response.total, "total"),
    hasMore: boolean(response.has_more, "has more"),
  };
}

/** Normalizes a snapshot-bound CAD bounds query response.
 * @param value Raw transport response.
 * @returns Typed page with the queried bounds and completeness metadata.
 * @throws When query metadata or its entity page is malformed.
 */
export function normalizeCadBoundsQueryResponse(value: unknown): CadBoundsQueryResponse {
  const response = record(value, "CAD bounds query");
  const bounds = record(response.bounds, "CAD bounds");
  const entities = normalizeCadEntitiesResponse(response);
  return {
    ...entities,
    space: normalizeSpace(response.space),
    bounds: {
      minX: string(bounds.min_x, "minimum x"),
      minY: string(bounds.min_y, "minimum y"),
      maxX: string(bounds.max_x, "maximum x"),
      maxY: string(bounds.max_y, "maximum y"),
    },
    unboundedEntities: number(response.unbounded_entities, "unbounded entity count"),
    complete: boolean(response.complete, "complete"),
  };
}

/** Normalizes externally tagged CAD drawing spaces.
 * @param value Raw Rust enum value.
 * @returns Stable wire drawing space.
 * @throws When the enum tag or its identity is invalid.
 */
function normalizeSpace(value: unknown): CadDrawingSpace {
  if (value === "Model") return value;
  const space = record(value, "CAD drawing space");
  if (Object.keys(space).length === 1 && typeof space.Paper === "string") {
    return { Paper: space.Paper };
  }
  if (Object.keys(space).length === 1 && typeof space.Named === "string") {
    return { Named: space.Named };
  }
  throw new TypeError("CAD drawing space is invalid");
}

/** Normalizes an externally tagged semantic CAD geometry value.
 * @param value Raw Rust geometry enum value.
 * @returns Typed geometry with its stable variant tag.
 * @throws When the value is not one recognized semantic geometry variant.
 */
function normalizeGeometry(value: unknown): CadGeometry {
  const geometry = record(value, "CAD geometry");
  const tags = ["Line", "Polyline", "Circle", "Arc", "Text", "Dimension", "BlockReference"];
  const keys = Object.keys(geometry);
  const tag = keys[0];
  if (keys.length !== 1 || tag === undefined || !tags.includes(tag)) {
    throw new TypeError("CAD geometry variant is invalid");
  }
  const fields = record(geometry[tag], "CAD geometry fields");
  switch (tag) {
    case "Line":
      return {
        Line: {
          start: normalizeCoordinate(fields.start),
          end: normalizeCoordinate(fields.end),
        },
      };
    case "Polyline":
      if (!Array.isArray(fields.vertices)) throw new TypeError("CAD vertices must be an array");
      return {
        Polyline: {
          vertices: fields.vertices.map(normalizeCoordinate),
          closed: boolean(fields.closed, "polyline closed"),
        },
      };
    case "Circle":
      return {
        Circle: {
          center: normalizeCoordinate(fields.center),
          radius: string(fields.radius, "circle radius"),
        },
      };
    case "Arc":
      return {
        Arc: {
          center: normalizeCoordinate(fields.center),
          radius: string(fields.radius, "arc radius"),
          start_angle: string(fields.start_angle, "arc start angle"),
          end_angle: string(fields.end_angle, "arc end angle"),
        },
      };
    case "Text":
      return {
        Text: {
          position: normalizeCoordinate(fields.position),
          value: string(fields.value, "text value"),
          height: string(fields.height, "text height"),
        },
      };
    case "Dimension":
      if (!Array.isArray(fields.definition_points)) {
        throw new TypeError("dimension definition points must be an array");
      }
      return {
        Dimension: {
          definition_points: fields.definition_points.map(normalizeCoordinate),
          ...(fields.text == null ? {} : { text: string(fields.text, "dimension text") }),
        },
      };
    case "BlockReference":
      if (!Array.isArray(fields.scale) || fields.scale.length !== 3) {
        throw new TypeError("block scale must contain three coordinates");
      }
      return {
        BlockReference: {
          definition: string(fields.definition, "block definition"),
          insertion: normalizeCoordinate(fields.insertion),
          scale: fields.scale.map((item) => string(item, "block scale")) as [
            string,
            string,
            string,
          ],
          rotation: string(fields.rotation, "block rotation"),
        },
      };
    default:
      throw new TypeError("CAD geometry variant is unsupported");
  }
}

/** Normalizes one exact decimal coordinate tuple.
 * @param value Raw Rust coordinate value.
 * @returns Typed x, y, and z decimal strings.
 * @throws When any coordinate ordinate is missing or non-string.
 */
function normalizeCoordinate(value: unknown): { x: string; y: string; z: string } {
  const coordinate = record(value, "CAD coordinate");
  return {
    x: string(coordinate.x, "coordinate x"),
    y: string(coordinate.y, "coordinate y"),
    z: string(coordinate.z, "coordinate z"),
  };
}

/** Normalizes an adapter-neutral style reference.
 * @param value Raw style value.
 * @returns Typed style key and adapter.
 * @throws When required style strings are missing.
 */
function normalizeStyle(value: unknown): CadEntityStyleRef {
  const style = record(value, "CAD style");
  return { key: string(style.key, "style key"), adapter: string(style.adapter, "style adapter") };
}

/** Normalizes a Rust CAD candidate response.
 * @param value Raw transport response.
 * @returns Reviewable semantic CAD candidate metadata.
 * @throws When a required response field or validation state is malformed.
 */
export function normalizeCadLineEditResponse(value: unknown): CadLineEditResponse {
  const response = record(value, "CAD candidate");
  const validationValue = record(response.validation, "CAD validation state");
  const state = string(validationValue.state, "validation state");
  const validation =
    state === "pending"
      ? { status: "pending" as const }
      : state === "passed"
        ? { status: "passed" as const }
        : state === "failed" && Array.isArray(validationValue.diagnostics)
          ? {
              status: "failed" as const,
              diagnostics: validationValue.diagnostics.map((item) => string(item, "diagnostic")),
            }
          : undefined;
  if (validation === undefined) throw new TypeError("CAD validation state is invalid");
  return {
    candidateId: string(response.candidate, "candidate id"),
    project: string(response.project, "project"),
    baseSnapshot: string(response.base_snapshot, "base snapshot"),
    path: string(response.path, "source path"),
    validation,
  };
}

/** Narrows unknown JSON values to object records.
 * @param value Value to inspect.
 * @param label Field description for errors.
 * @returns Object record.
 * @throws When value is not a non-null object.
 */
function record(value: unknown, label: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value as Record<string, unknown>;
}

/** Validates a required CAD response string.
 * @param value Value to inspect.
 * @param label Field description for errors.
 * @returns String value.
 * @throws When value is not a string.
 */
function string(value: unknown, label: string): string {
  if (typeof value !== "string") throw new TypeError(`${label} must be a string`);
  return value;
}

/** Validates one required boolean response field.
 * @param value Value to inspect.
 * @param label Field description for errors.
 * @returns Boolean value.
 * @throws When value is not boolean.
 */
function boolean(value: unknown, label: string): boolean {
  if (typeof value !== "boolean") throw new TypeError(`${label} must be a boolean`);
  return value;
}

/** Validates one required finite number response field.
 * @param value Value to inspect.
 * @param label Field description for errors.
 * @returns Finite number value.
 * @throws When value is not a finite number.
 */
function number(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new TypeError(`${label} must be a finite number`);
  }
  return value;
}
