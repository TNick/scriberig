/** @file Provides server-backed project catalog and scan onboarding. */
import {
  Alert,
  Box,
  Button,
  Checkbox,
  Chip,
  FormControlLabel,
  List,
  ListItem,
  ListItemText,
  Paper,
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
      <Typography color="text.secondary" sx={{ maxWidth: 640, mb: 3 }}>
        Open a knowledge workspace or scan a project folder to bring its sources and structure into
        Scriberig.
      </Typography>
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
      <Paper variant="outlined" sx={{ p: { xs: 1, sm: 2 }, mb: 3, maxWidth: 980 }}>
        <Typography variant="h6" sx={{ px: 1, pt: 0.5 }}>
          Your workspaces
        </Typography>
        {projects.length === 0 ? (
          <Typography color="text.secondary" sx={{ p: 1.5 }}>
            No workspaces yet. Scan a project folder below to get started.
          </Typography>
        ) : (
          <List aria-label="Projects">
            {projects.map((project) => (
              <Box
                component="li"
                key={project.id}
                sx={{
                  listStyle: "none",
                  display: "flex",
                  alignItems: "center",
                  px: 1.5,
                  py: 1,
                  borderTop: 1,
                  borderColor: "divider",
                  gap: 2,
                }}
              >
                <ListItemText primary={project.name} secondary={project.path} />
                <Chip size="small" label={project.status} variant="outlined" />
                {project.status === "imported" && (
                  <Button onClick={() => void openProject(project)}>Open {project.name}</Button>
                )}
              </Box>
            ))}
          </List>
        )}
      </Paper>
      <Paper variant="outlined" sx={{ p: { xs: 2, sm: 3 }, maxWidth: 980 }}>
        <Typography variant="h6" sx={{ mb: 0.5 }}>
          Add a project
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2.5 }}>
          Scan first, review discovered sources, then import the project.
        </Typography>
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
      </Paper>
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
