/** @file Defines the top-level reference Scriberig editor component. */

import { useEffect, useState, type ReactElement } from "react";

/**
 * Render the reference Scriberig editor scaffold and API readiness.
 *
 * @returns The current editor application shell.
 */
export function App(): ReactElement {
  const [apiStatus, setApiStatus] = useState("Checking local API…");

  useEffect(() => {
    let mounted = true;
    void fetch("/api/health")
      .then((response) => {
        if (!response.ok) {
          return null;
        }
        return response.json() as Promise<{
          status: string;
          database: string;
        } | null>;
      })
      .then((result) => {
        if (mounted && result) {
          setApiStatus(`API ${result.status}; database ${result.database}`);
        } else if (mounted) {
          setApiStatus("Local API is unavailable");
        }
      })
      .catch(() => {
        if (mounted) {
          setApiStatus("Local API is unavailable");
        }
      });

    return () => {
      mounted = false;
    };
  }, []);

  return (
    <main>
      <h1>Scriberig</h1>
      <p>Editor scaffold</p>
      <p role="status">{apiStatus}</p>
    </main>
  );
}
