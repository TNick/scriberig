/** @file Composes bounded geospatial layer browsing, mapping, and candidate review. */
import { Alert, Box, Button, MenuItem, Stack, TextField, Typography } from "@mui/material";
import { useEffect, useMemo, useState, type ReactElement } from "react";
import type { GeoFeatureView, GeoLayerView, QgisProjectResponse } from "@scriberig/protocol";
import type { ScriberigClient } from "@scriberig/client";
import { GeoMapCanvas, ProjectCandidateReview, QgisProjectView } from "@scriberig/components";
import {
  GeoPageController,
  ProjectCandidateController,
  QgisProjectController,
} from "@scriberig/model";
import { FeatureEditor, LayerSummary, crsLabel } from "./GeoFeatureEditor";
import {
  attributeText,
  extentBounds,
  pageDiagnostics,
  qgisReferenceIsBrowsable,
} from "./geo-view-helpers";

/** Renders bounded GeoPackage layers and exact-snapshot feature candidates.
 * @param props Shared project client, snapshot, and semantic selection callbacks.
 * @returns GeoPackage source selector, layer explorer, bounded map, and candidate review.
 */
export function GeoView({
  client,
  project,
  snapshot,
  onSelectFeature,
  onCandidateApplied,
}: {
  /** Shared semantic client. */
  client: ScriberigClient;
  /** Active semantic project identity. */
  project: string;
  /** Exact immutable project snapshot. */
  snapshot: string;
  /** Publishes an exact semantic feature selection. */
  onSelectFeature: (feature: GeoFeatureView, snapshot: string, layer: string) => void;
  /** Publishes the successor snapshot after shared candidate apply. */
  onCandidateApplied?: (snapshot: string) => void;
}): ReactElement {
  const [paths, setPaths] = useState<string[]>([]);
  const [path, setPath] = useState("");
  const [qgisPaths, setQgisPaths] = useState<string[]>([]);
  const [qgisPath, setQgisPath] = useState("");
  const [qgisProject, setQgisProject] = useState<QgisProjectResponse>();
  const [preferredLayerId, setPreferredLayerId] = useState("");
  const [layers, setLayers] = useState<GeoLayerView[]>([]);
  const [layerId, setLayerId] = useState("");
  const [features, setFeatures] = useState<GeoFeatureView[]>([]);
  const [queryDiagnostics, setQueryDiagnostics] = useState<string[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [offset, setOffset] = useState(0);
  const [selected, setSelected] = useState<GeoFeatureView>();
  const [field, setField] = useState("");
  const [value, setValue] = useState("");
  const [geometryText, setGeometryText] = useState("");
  const [candidateId, setCandidateId] = useState<string>();
  const [candidateSummary, setCandidateSummary] = useState<string>();
  const [error, setError] = useState<string>();
  const controller = useMemo(
    () => (path ? new GeoPageController(client, project, path, snapshot) : undefined),
    [client, project, path, snapshot],
  );
  const qgisController = useMemo(
    () =>
      qgisPath === "" ? undefined : new QgisProjectController(client, project, qgisPath, snapshot),
    [client, project, qgisPath, snapshot],
  );
  const candidateController = useMemo(
    () =>
      candidateId === undefined ? undefined : new ProjectCandidateController(client, candidateId),
    [client, candidateId],
  );
  useEffect(() => {
    const abort = new AbortController();
    setPaths([]);
    setPath("");
    setQgisPaths([]);
    setQgisPath("");
    void client
      .inspectArtifacts({ project, offset: 0, limit: 100 }, abort.signal)
      .then((page) => {
        if (abort.signal.aborted) return;
        const sources = page.artifacts
          .filter((artifact) => artifact.adapter === "geopackage")
          .flatMap((artifact) => artifact.members.map((member) => member.projectPath))
          .filter((candidate): candidate is string => candidate !== undefined);
        const unique = [...new Set(sources)].sort();
        const projects = page.artifacts
          .filter((artifact) => artifact.adapter === "qgis")
          .flatMap((artifact) => artifact.members.map((member) => member.projectPath))
          .filter((candidate): candidate is string => candidate !== undefined);
        const uniqueProjects = [...new Set(projects)].sort();
        setPaths(unique);
        setPath(unique[0] ?? "");
        setQgisPaths(uniqueProjects);
        setQgisPath(uniqueProjects[0] ?? "");
        setError(undefined);
      })
      .catch((reason: unknown) => {
        if (!abort.signal.aborted) setError(String(reason));
      });
    return () => abort.abort();
  }, [client, project, snapshot]);
  useEffect(() => {
    if (qgisController === undefined) {
      setQgisProject(undefined);
      return;
    }
    const abort = new AbortController();
    setQgisProject(undefined);
    void qgisController
      .read(abort.signal)
      .then((result) => {
        if (!abort.signal.aborted) setQgisProject(result);
      })
      .catch((reason: unknown) => {
        if (!abort.signal.aborted) setError(String(reason));
      });
    return () => abort.abort();
  }, [qgisController]);
  useEffect(() => {
    if (controller === undefined) return;
    const abort = new AbortController();
    setLayers([]);
    setLayerId("");
    setFeatures([]);
    setSelected(undefined);
    setCandidateId(undefined);
    setCandidateSummary(undefined);
    void controller
      .readCatalog(0, 100, abort.signal)
      .then((catalog) => {
        if (abort.signal.aborted) return;
        setLayers(catalog.layers);
        setLayerId(
          catalog.layers.find((item) => item.id === preferredLayerId)?.id ??
            catalog.layers[0]?.id ??
            "",
        );
        setError(undefined);
      })
      .catch((reason: unknown) => {
        if (!abort.signal.aborted) setError(String(reason));
      });
    return () => abort.abort();
  }, [controller, preferredLayerId]);
  const layer = layers.find((item) => item.id === layerId);
  useEffect(() => {
    if (controller === undefined || layer === undefined) return;
    const abort = new AbortController();
    setOffset(0);
    setFeatures([]);
    setSelected(undefined);
    const load =
      layer.extent !== undefined && layer.crs !== undefined
        ? controller.queryEnvelope(layer.id, extentBounds(layer), 0, abort.signal)
        : controller.readFeatures(layer.id, 0, abort.signal);
    void load
      .then((page) => {
        if (abort.signal.aborted) return;
        setFeatures(page.features);
        setHasMore(page.hasMore);
        setQueryDiagnostics(pageDiagnostics(page));
        setError(undefined);
      })
      .catch((reason: unknown) => {
        if (!abort.signal.aborted) setError(String(reason));
      });
    return () => abort.abort();
  }, [controller, layer]);
  const activeField = layer?.fields.find((item) => item.name === field);
  return (
    <Box component="section" aria-label="Geospatial project view">
      <Typography variant="h5">Geospatial layers</Typography>
      {error !== undefined ? <Alert severity="error">{error}</Alert> : null}
      {paths.length === 0 ? <p>No GeoPackage sources are available in this project.</p> : null}
      {paths.length > 0 ? (
        <TextField
          select
          label="GeoPackage source"
          value={path}
          onChange={(event) => setPath(event.target.value)}
          sx={{ minWidth: 280 }}
        >
          {paths.map((source) => (
            <MenuItem key={source} value={source}>
              {source}
            </MenuItem>
          ))}
        </TextField>
      ) : null}
      {qgisPaths.length > 0 ? (
        <TextField
          select
          label="QGIS project"
          value={qgisPath}
          onChange={(event) => setQgisPath(event.target.value)}
          sx={{ minWidth: 280, ml: 2 }}
        >
          {qgisPaths.map((source) => (
            <MenuItem key={source} value={source}>
              {source}
            </MenuItem>
          ))}
        </TextField>
      ) : null}
      {qgisProject && qgisController ? (
        <QgisProjectView
          project={qgisProject}
          controller={qgisController}
          canBrowse={qgisReferenceIsBrowsable}
          onBrowse={(reference) => {
            if (reference.layer === undefined || reference.resolvedPath === undefined) return;
            setPreferredLayerId(reference.layer);
            setPath(reference.resolvedPath);
          }}
          onCandidate={(candidate, summary) => {
            setCandidateId(candidate);
            setCandidateSummary(summary);
            setError(undefined);
          }}
          onError={setError}
        />
      ) : null}
      {layers.length > 0 ? (
        <TextField
          select
          label="Layer"
          value={layerId}
          onChange={(event) => setLayerId(event.target.value)}
          sx={{ minWidth: 280, ml: 2 }}
        >
          {layers.map((item) => (
            <MenuItem key={item.id} value={item.id}>
              {item.name} · {item.featureCount} features
            </MenuItem>
          ))}
        </TextField>
      ) : null}
      {layer ? <LayerSummary layer={layer} /> : null}
      {queryDiagnostics.map((diagnostic) => (
        <Typography key={diagnostic} role="note">
          {diagnostic}
        </Typography>
      ))}
      {layer ? (
        <GeoMapCanvas
          features={features}
          crsLabel={crsLabel(layer)}
          onMapCoordinate={(coordinates) => {
            if (selected === undefined) return;
            setGeometryText(JSON.stringify({ kind: "point", coordinates }, null, 2));
          }}
          onSelectFeature={(feature) => {
            setSelected(feature);
            setCandidateId(undefined);
            setCandidateSummary(undefined);
            setField(layer.fields[0]?.name ?? "");
            setValue(attributeText(feature, layer.fields[0]?.name ?? ""));
            setGeometryText(JSON.stringify(feature.geometry ?? null, null, 2));
            onSelectFeature(feature, snapshot, layer.id);
          }}
        />
      ) : null}
      {layer && features.length > 0 ? (
        <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
          <Button disabled={offset === 0} onClick={() => void loadPage(Math.max(0, offset - 100))}>
            Previous features
          </Button>
          <Typography role="status">
            Showing {offset + 1}–{offset + features.length}
          </Typography>
          <Button disabled={!hasMore} onClick={() => void loadPage(offset + features.length)}>
            Next features
          </Button>
        </Stack>
      ) : null}
      {selected && layer ? (
        <FeatureEditor
          controller={controller}
          layer={layer}
          feature={selected}
          crsLabel={crsLabel(layer)}
          field={field}
          value={value}
          geometryText={geometryText}
          activeField={activeField}
          onField={setField}
          onValue={setValue}
          onGeometry={setGeometryText}
          onCandidate={(candidate, summary) => {
            setCandidateId(candidate);
            setCandidateSummary(summary);
          }}
          onError={(message) => setError(message)}
        />
      ) : null}
      {candidateController ? (
        <ProjectCandidateReview
          key={candidateId}
          controller={candidateController}
          {...(candidateSummary === undefined ? {} : { summary: candidateSummary })}
          onApplied={(result) => onCandidateApplied?.(result.successor)}
        />
      ) : null}
    </Box>
  );

  /** Loads another exact-snapshot bounded page for the selected layer.
   * @param nextOffset Zero-based offset for the next feature page.
   * @returns Completion after page state has been refreshed.
   */
  async function loadPage(nextOffset: number): Promise<void> {
    if (controller === undefined || layer === undefined) return;
    try {
      const page =
        layer.extent !== undefined && layer.crs !== undefined
          ? await controller.queryEnvelope(layer.id, extentBounds(layer), nextOffset)
          : await controller.readFeatures(layer.id, nextOffset);
      setFeatures(page.features);
      setHasMore(page.hasMore);
      setQueryDiagnostics(pageDiagnostics(page));
      setOffset(nextOffset);
      setSelected(undefined);
    } catch (reason) {
      setError(String(reason));
    }
  }
}
