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
import type { ScriberigClient } from "@scriberig/client";

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

/** Renders the project catalog and reviewable filesystem scan.
 * @returns The project manager view.
 * @throws Network failures are rendered as an error state.
 */
export function ProjectManager({ client }: { client: ScriberigClient }): ReactElement {
  const [projects, setProjects] = useState<Project[]>([]);
  const [path, setPath] = useState("");
  const [name, setName] = useState("");
  const [gitTracked, setGitTracked] = useState(false);
  const [scan, setScan] = useState<ScanResult>();
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState<string>();

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
    }
  }

  /** Imports the reviewed scan into the selected project snapshot.
   * @returns A promise completed after import output is applied.
   * @throws Network and server failures are rendered as an error state.
   */
  async function importProject(): Promise<void> {
    if (scan === undefined) return;
    setError(undefined);
    setImporting(true);
    try {
      const value = await client.project<{ project: Project }>("import", {
        project: scan.project.id,
      });
      setProjects((current) => [
        ...current.filter((project) => project.id !== value.project.id),
        value.project,
      ]);
      setScan(undefined);
    } catch (reason: unknown) {
      setError(String(reason));
    } finally {
      setImporting(false);
    }
  }

  return (
    <Box>
      <Typography variant="h5">Projects</Typography>
      {error !== undefined && <Alert severity="error">{error}</Alert>}
      <List aria-label="Projects">
        {projects.map((project) => (
          <ListItem key={project.id}>
            <ListItemText
              primary={`${project.name} (${project.status})`}
              secondary={project.path}
            />
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
