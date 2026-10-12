/** @file Provides server-backed project catalog and scan onboarding. */
import {
  Alert,
  Box,
  Button,
  Checkbox,
  FormControlLabel,
  List,
  ListItem,
  ListItemText,
  TextField,
  Typography,
} from "@mui/material";
import { useEffect, useState, type FormEvent, type ReactElement } from "react";
import { ServerProtocolError, type ScriberigClient } from "@scriberig/client";

interface Project {
  id: string;
  name: string;
  path: string;
  status: string;
  snapshot?: string;
}

interface ScanResult {
  project: Project;
  files: Array<{ path: string; adapter: string; status: string; bytes: number }>;
  gitTracked: boolean;
}

interface ImportDiagnostic {
  path: string;
  adapter: string;
  stage: string;
  code: string;
  message: string;
  retryable: boolean;
}

/** Renders the project catalog and reviewable filesystem scan.
 * @returns The project manager view.
 * @throws Network failures are rendered as an error state.
 */
export function ProjectManager({
  client,
  onActivate,
}: {
  client: ScriberigClient;
  onActivate: (projectId: string) => Promise<void>;
}): ReactElement {
  const [projects, setProjects] = useState<Project[]>([]);
  const [path, setPath] = useState("");
  const [name, setName] = useState("");
  const [gitTracked, setGitTracked] = useState(false);
  const [scan, setScan] = useState<ScanResult>();
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState<string>();
  const [diagnostics, setDiagnostics] = useState<ImportDiagnostic[]>([]);

  useEffect(() => {
    void client
      .project<{ projects: Project[] }>("list", {})
      .then((value) => setProjects(value.projects))
      .catch((reason: unknown) => setError(String(reason)));
  }, [client]);

  /** Starts a server-side project scan.
   * @param event Form submission event.
   * @returns A promise completed after the scan response is applied.
   * @throws Network and server failures are rendered as an error state.
   */
  async function submit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    setError(undefined);
    setDiagnostics([]);
    setScan(undefined);
    try {
      const value = await client.project<ScanResult>("scan", {
        path,
        ...(name === "" ? {} : { name }),
        ...(gitTracked ? { gitTracked: true } : {}),
      });
      setScan(value);
      setProjects((current) => [
        ...current.filter((project) => project.id !== value.project.id),
        value.project,
      ]);
    } catch (reason: unknown) {
      setError(String(reason));
      if (reason instanceof ServerProtocolError) {
        const value = reason.error.details?.diagnostics;
        if (Array.isArray(value)) setDiagnostics(value as ImportDiagnostic[]);
      }
    }
  }

  /** Imports the reviewed scan into the selected project snapshot.
   * @returns A promise completed after import output is applied.
   * @throws Network and server failures are rendered as an error state.
   */
  async function importProject(): Promise<void> {
    if (scan === undefined) return;
    setError(undefined);
    setDiagnostics([]);
    setImporting(true);
    try {
      const value = await client.project<{ project: Project }>("import", {
        project: scan.project.id,
      });
      setProjects((current) => [
        ...current.filter((project) => project.id !== value.project.id),
        value.project,
      ]);
      await onActivate(value.project.id);
      setScan(undefined);
    } catch (reason: unknown) {
      setError(String(reason));
      if (reason instanceof ServerProtocolError) {
        const value = reason.error.details?.diagnostics;
        if (Array.isArray(value)) setDiagnostics(value as ImportDiagnostic[]);
      }
    } finally {
      setImporting(false);
    }
  }

  /** Opens an already imported project in this client workspace.
   * @param project Imported project to activate.
   * @returns Resolves after activation completes.
   */
  async function openProject(project: Project): Promise<void> {
    setError(undefined);
    try {
      await onActivate(project.id);
    } catch (reason: unknown) {
      setError(String(reason));
    }
  }

  return (
    <Box>
      <Typography variant="h5">Projects</Typography>
      {error !== undefined && <Alert severity="error">{error}</Alert>}
      {diagnostics.length > 0 && (
        <List aria-label="Import diagnostics">
          {diagnostics.map((diagnostic) => (
            <ListItem key={`${diagnostic.path}:${diagnostic.code}:${diagnostic.message}`}>
              <ListItemText
                primary={`${diagnostic.path} · ${diagnostic.adapter} · ${diagnostic.code}`}
                secondary={`${diagnostic.stage}: ${diagnostic.message}${diagnostic.retryable ? " (repairable)" : ""}`}
              />
            </ListItem>
          ))}
        </List>
      )}
      <List aria-label="Projects">
        {projects.map((project) => (
          <ListItem key={project.id}>
            <ListItemText
              primary={`${project.name} (${project.status})`}
              secondary={project.path}
            />
            {project.status === "imported" && (
              <Button onClick={() => void openProject(project)}>Open {project.name}</Button>
            )}
          </ListItem>
        ))}
      </List>
      <Box
        component="form"
        onSubmit={(event) => void submit(event)}
        sx={{ display: "grid", gap: 2, maxWidth: 640 }}
      >
        <TextField
          label="Server filesystem path"
          value={path}
          onChange={(event) => setPath(event.target.value)}
          required
        />
        <FormControlLabel
          control={
            <Checkbox
              checked={gitTracked}
              onChange={(event) => setGitTracked(event.target.checked)}
            />
          }
          label="Only Git-tracked files"
        />
        <TextField
          label="Project name"
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
        <Button type="submit" variant="contained">
          Scan project
        </Button>
      </Box>
      {scan !== undefined && (
        <Box sx={{ mt: 3 }}>
          <Typography variant="h6">Scan review</Typography>
          <Typography>
            {scan.files.length} files discovered for {scan.project.name}.
            {scan.gitTracked ? " Git-tracked files only." : " All scanned files."}
          </Typography>
          <Button
            sx={{ my: 1 }}
            variant="contained"
            onClick={() => void importProject()}
            disabled={importing}
          >
            {importing ? "Importing…" : "Import reviewed scan"}
          </Button>
          <List aria-label="Scan manifest">
            {scan.files.slice(0, 100).map((file) => (
              <ListItem key={file.path}>
                <ListItemText
                  primary={file.path}
                  secondary={`${file.adapter} · ${file.status} · ${file.bytes} bytes`}
                />
              </ListItem>
            ))}
          </List>
        </Box>
      )}
    </Box>
  );
}
