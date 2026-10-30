/** @file Presents paged artifact summaries from the active semantic project. */
import { Alert, Box, Button, CircularProgress, Stack, Typography } from "@mui/material";
import { useEffect, useMemo, useState, type ReactElement } from "react";
import type { ArtifactInspectResponse, ArtifactMembersResponse } from "@scriberig/protocol";
import { ArtifactInspector } from "@scriberig/components";
import type { ScriberigClient } from "@scriberig/client";
import { ArtifactPageController } from "@scriberig/model";

/** Renders artifact summaries with bounded pagination.
 * @param props Active project and shared API client.
 * @returns Artifact list, loading state, or inspection error.
 */
export function ArtifactsView({
  client,
  project,
}: {
  /** Shared semantic client. */
  client: ScriberigClient;
  /** Active project identity. */
  project: string;
}): ReactElement {
  const controller = useMemo(() => new ArtifactPageController(client, project), [client, project]);
  const [offset, setOffset] = useState(0);
  const [page, setPage] = useState<ArtifactInspectResponse>();
  const [error, setError] = useState<string>();
  const [loading, setLoading] = useState(false);
  const [selectedArtifact, setSelectedArtifact] = useState<string>();
  const [members, setMembers] = useState<ArtifactMembersResponse>();
  const [memberError, setMemberError] = useState<string>();
  const [memberLoading, setMemberLoading] = useState(false);
  useEffect(() => {
    const abortController = new AbortController();
    setLoading(true);
    setError(undefined);
    setPage(undefined);
    void controller
      .read(offset, abortController.signal)
      .then((value) => {
        if (!abortController.signal.aborted) setPage(value);
      })
      .catch((reason: unknown) => {
        if (!abortController.signal.aborted) setError(String(reason));
      })
      .finally(() => {
        if (!abortController.signal.aborted) setLoading(false);
      });
    return () => abortController.abort();
  }, [controller, offset]);

  useEffect(() => {
    if (selectedArtifact === undefined) {
      setMembers(undefined);
      return;
    }
    const abortController = new AbortController();
    setMemberLoading(true);
    setMemberError(undefined);
    void client
      .inspectArtifactMembers(
        { project, artifact: selectedArtifact, offset: 0, limit: 20 },
        abortController.signal,
      )
      .then((value) => {
        if (!abortController.signal.aborted) setMembers(value);
      })
      .catch((reason: unknown) => {
        if (!abortController.signal.aborted) setMemberError(String(reason));
      })
      .finally(() => {
        if (!abortController.signal.aborted) setMemberLoading(false);
      });
    return () => abortController.abort();
  }, [client, project, selectedArtifact]);

  return (
    <Box>
      <Typography variant="h5">Project artifacts</Typography>
      {error !== undefined ? <Alert severity="error">{error}</Alert> : null}
      {loading ? <CircularProgress aria-label="Loading artifacts" /> : null}
      {page?.artifacts.length === 0 && !loading ? <p>No artifacts found.</p> : null}
      <Stack spacing={2}>
        {page?.artifacts.map((artifact) => (
          <ArtifactInspector
            key={artifact.id}
            artifact={artifact}
            onInspectMembers={(artifactId) => setSelectedArtifact(artifactId)}
          />
        ))}
      </Stack>
      {selectedArtifact !== undefined ? (
        <Box aria-label="Artifact member evidence" sx={{ mt: 2 }}>
          <Typography variant="h6">Artifact members</Typography>
          {memberLoading ? <CircularProgress aria-label="Loading artifact members" /> : null}
          {memberError !== undefined ? <Alert severity="error">{memberError}</Alert> : null}
          <ul>
            {members?.members.map((member) => (
              <li key={member.key}>
                {member.key} · {member.projectPath ?? member.containerPart ?? "package member"} ·{" "}
                {member.digest}
              </li>
            ))}
          </ul>
          {members?.nextOffset !== undefined ? (
            <Button
              disabled={memberLoading}
              onClick={() => {
                const next = members.nextOffset;
                if (next === undefined) return;
                setMemberLoading(true);
                void client
                  .inspectArtifactMembers({
                    project,
                    artifact: selectedArtifact,
                    offset: next,
                    limit: 20,
                  })
                  .then((page) =>
                    setMembers((current) =>
                      current
                        ? {
                            ...page,
                            members: [...current.members, ...page.members],
                          }
                        : page,
                    ),
                  )
                  .catch((reason: unknown) => setMemberError(String(reason)))
                  .finally(() => setMemberLoading(false));
              }}
            >
              Load more members
            </Button>
          ) : null}
          {members !== undefined ? (
            <Typography role="status">
              Showing {members.members.length} of {members.total} members
            </Typography>
          ) : null}
        </Box>
      ) : null}
      {page !== undefined ? (
        <Stack direction="row" spacing={1} sx={{ mt: 2 }}>
          <Button
            disabled={loading || offset === 0}
            onClick={() => setOffset(Math.max(0, offset - 20))}
          >
            Previous
          </Button>
          <Typography role="status">
            {page.total === 0
              ? "0 of 0"
              : `${offset + 1}–${Math.min(offset + page.artifacts.length, page.total)} of ${page.total}`}
          </Typography>
          <Button
            disabled={loading || page.nextOffset === undefined}
            onClick={() => setOffset(page.nextOffset ?? offset)}
          >
            Next
          </Button>
        </Stack>
      ) : null}
    </Box>
  );
}
