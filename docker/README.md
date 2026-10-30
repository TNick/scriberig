# Docker usage

The images run the same `srig` executable and integrated browser application as
the local development setup. Docker Desktop or Docker Engine with Compose is
required. Build from the repository root:

```text
make docker-cli-image
make docker-web-image
```

The equivalent commands are `docker compose build api web`, or individual
`docker build -f docker/Dockerfile.cli -t scriberig-cli:local .` and
`docker build -f docker/Dockerfile.web -t scriberig-web:local .` commands.

## Web application

From the repository root, choose an existing host project and start the stack.
If `SCRIBERIG_PROJECT` is unset, the repository itself is mounted. The API can
scan only paths under `/workspace` inside the container.

PowerShell:

```powershell
$env:SCRIBERIG_PROJECT = 'D:\projects\my-project'
docker compose up --build -d
```

Linux or macOS:

```sh
export SCRIBERIG_PROJECT="$HOME/projects/my-project"
docker compose up --build -d
```

Open `http://localhost:8080`, and use `/workspace` as the project path when
onboarding. Set `SCRIBERIG_WEB_PORT` before starting to change the host port.
The browser and API use the same origin; the API is not published separately.
This is a local, single-user deployment and has no network authentication.
Do not expose its port to an untrusted network.

```text
docker compose ps
docker compose logs api web
docker compose down
```

`make docker-web` builds and starts the stack, and `make docker-web-stop` stops
it. The named Compose volume persists the SQLite database across container
replacement and normal `down` operations. Project files remain on the host.
Keep the same host project mounted at `/workspace` when reopening a registered
project; use the existing relink operation if its container path changes.

The API runs as Linux UID 10001. Ensure the mounted project is readable and, for
materialization, writable by that user. On native Linux, adjust host ownership
or permissions to grant access without making the project world-writable.
Docker Desktop mount permissions depend on its file-sharing settings. A missing
host path fails startup instead of silently creating an empty directory. Consult
`docker compose logs api` for database and project access errors.

## CLI and TUI

The CLI image starts `srig` and accepts its usual arguments. Mount both the
project and a named volume for the database. Use the same paths for every run.

PowerShell:

```powershell
docker run --rm scriberig-cli:local --help
docker run --rm --mount "type=bind,source=$((Get-Location).Path),target=/workspace" --mount type=volume,source=scriberig-cli-state,target=/var/lib/scriberig scriberig-cli:local project scan /workspace --db /var/lib/scriberig/scriberig.sqlite
docker run --rm --mount type=volume,source=scriberig-cli-state,target=/var/lib/scriberig scriberig-cli:local project list --db /var/lib/scriberig/scriberig.sqlite
```

Linux or macOS:

```sh
docker run --rm scriberig-cli:local --help
docker run --rm -v "$PWD:/workspace" -v scriberig-cli-state:/var/lib/scriberig scriberig-cli:local project scan /workspace --db /var/lib/scriberig/scriberig.sqlite
docker run --rm -v scriberig-cli-state:/var/lib/scriberig scriberig-cli:local project list --db /var/lib/scriberig/scriberig.sqlite
docker run --rm -it -v "$PWD:/workspace" -v scriberig-cli-state:/var/lib/scriberig scriberig-cli:local tui --db /var/lib/scriberig/scriberig.sqlite --project /workspace --import
```

For a PowerShell TUI, add `-it` and use the same two mounts as the scan command.
Use the project ID printed by `project scan` with `project import ID --db
/var/lib/scriberig/scriberig.sqlite`. Scan and import use the container's path,
not the host path. `--git-tracked` requires the mounted Git metadata and Git
installed in the image. On Linux, use `--user "$(id -u):$(id -g)"` when host
file ownership requires it; the selected user must also be able to write the
state volume.

The CLI volume in these examples is independent of the Compose volume. Avoid
sharing a SQLite file between concurrently running containers over network
filesystems.

## Provider credentials

The image does not copy host credentials or provide a desktop OS keyring. For
headless use, configure an environment or owner-only mounted-file credential
and its exact provider and container project-path binding as described in
[Managing secrets](../SECRETS.md). Supply configuration and secrets at runtime;
never put credentials in the Dockerfile, image, command arguments, or committed
environment files. Mount any persistent configuration needed by the container.
