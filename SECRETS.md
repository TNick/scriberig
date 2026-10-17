# Managing Scriberig secrets

Scriberig stores provider credentials in the current user's operating-system
credential store. Configuration contains an indirect reference and ordinary
provider settings. It never needs to contain an API key.

## Local setup

Store a credential using a hidden prompt, then authorize its exact provider
endpoint and project directory:

```text
srig secrets set providers/work
srig secrets bind providers/work --base-url https://provider.example/v1 --project-root PATH
```

Place a reference in your user or project configuration:

```yaml
base_url: https://provider.example/v1
model: configured-model
credential:
  source: keyring
  name: providers/work
```

The same object is supported in JSON. `credential` is one atomic property:
system, user, project, environment, and explicit CLI layers replace its entire
reference. Its internal fields are never merged across layers.

```text
srig run "Summarize this project" --credential-ref keyring:providers/work
srig config show --format json
srig secrets list --format json
srig secrets status providers/work --format json
```

`config show` reports references and provenance without opening the credential
store. `list` reads only protected metadata. `status` checks store availability
without contacting a provider. Management commands are local and are not
available to agents or through the browser protocol.

## Source selection and compatibility

Environment credentials remain supported:

```yaml
credential:
  source: environment
  name: PROVIDER_API_KEY
```

Authorize them through an explicit binding:

```text
srig secrets bind environment:PROVIDER_API_KEY --base-url https://provider.example/v1 --project-root PATH
```

The legacy `api_key_env` property, `SRIG_API_KEY_ENV`, and `--api-key-env` option
normalize to an environment reference. Supplying both legacy and canonical
spellings in one layer is an error. `SRIG_CREDENTIAL` accepts a JSON reference
object or `null`. Environment values are never interpolated into configuration.

An explicit local CLI pair `--api-key-env NAME --base-url URL` authorizes that
environment reference for this invocation and working directory only. A
project-selected endpoint never grants authority to send an inherited key.
Administrator restrictions still apply to explicit CLI pairs.

Use `credential: null` in YAML/JSON, `SRIG_CREDENTIAL=null`, or
`--credential-ref none` to clear an inherited reference. A non-null reference
requires a configured provider base URL.

## Headless deployments

Use a secret injected by your deployment manager, either as a dedicated
environment variable or an owner-only mounted file. Reserved toolchain variables
such as `PATH`, `HOME`, and `APPDATA` cannot be credential references.

```yaml
credential:
  source: file
  path: /run/secrets/provider-api-key
```

```text
srig secrets bind file:/run/secrets/provider-api-key --base-url https://provider.example/v1 --project-root PATH
```

Mounted files must be absolute, regular, and readable by the service identity.
On Unix use owner-only permissions such as `0400` or `0600`; group or public
access is rejected. On Windows only the current user, SYSTEM, and Administrators
may receive access. Symlinks, Windows reparse points, multiline input, empty
values, and values larger than 64 KiB are rejected. One terminal line ending
is removed. No backend silently falls back to plaintext storage.

`srig secrets set NAME --stdin` reads bounded input from a trusted pipe. Do not
put a credential in a shell command, argument, command substitution, transcript,
or committed `.env` file. The management CLI writes only the OS store; mounted
files and environment variables remain managed by the deployment system.

Local `run` and `agent` invocations use local execution bindings even in CI.
`secrets bind --service` is for embedding/service compositions that explicitly
supply a service execution context. Browser clients never receive provider
credentials.

## Authorization and platform storage

Windows uses Credential Manager with local-machine persistence. macOS uses
Keychain, and Linux uses an unlocked Secret Service collection. Store failures
are reported without selecting a different source.

Bindings match the exact normalized base URL and canonical working directory,
purpose `model_provider`, and execution mode. HTTPS is required. Numeric
loopback HTTP is permitted only with an explicit matching binding. URLs cannot
contain user information, queries, or fragments. Redirects are disabled.

User metadata is stored at the conventional user configuration directory under
`secrets/registry.json`; updates use an exclusive OS lock and atomic replacement.
This registry contains aliases, references, bindings, and protected file paths,
but no values or value hashes. Secret-file exclusions are retained after
unbinding so removing authorization cannot accidentally expose a credential.

Administrators may install `secrets/policy.json` under the conventional system
configuration directory. Its JSON shape is `{"bindings": [...]}`, using the
same binding objects shown by `secrets list`. An installed system policy is an
additional mandatory allowlist; an empty list denies all use. On Unix it must
be root-owned and not writable by group/others. On Windows it must be owned by
SYSTEM or Administrators and grant mutation only to those administrators.
Users may receive read access. Invalid policy files fail closed.

## Rotation and deletion

```text
srig secrets set providers/work --replace
srig secrets status providers/work
srig secrets unbind providers/work --base-url https://provider.example/v1 --project-root PATH
srig secrets delete providers/work
```

Resolve credentials again before every provider request. Replacement therefore
affects subsequent requests, including nested branches. Requests already in
flight may complete with the old value. To rotate, provision a replacement at
the issuer, replace the stored value, validate deliberately, then revoke the
old credential at the issuer. Local deletion does not perform issuer-side
revocation.

Mutation first disables affected bindings. Backend or metadata failures may
leave an orphaned credential or disabled bindings; exit status `27` reports
this partial state. Inspect status and restore bindings explicitly after
recovering. No failure prints backend diagnostic payloads or values.

| Exit status | Meaning |
| --- | --- |
| 20 | Missing credential. |
| 21 | Locked/inaccessible native store. |
| 22 | Backend unavailable. |
| 23 | Policy or filesystem access denied. |
| 24 | Backend unsupported. |
| 25 | Invalid input/reference/policy. |
| 26 | Existing credential requires explicit replacement. |
| 27 | Partial lifecycle operation; affected authorization is disabled. |

## Project execution and exposure recovery

Scans, persisted-manifest imports, projections, candidates, and validation
copies exclude known credential files and registered secret paths. Examples
include `.env`, `.env.*`, private-key files, `.npmrc`, `.netrc`, and credential
directories. Explicit templates such as `.env.example` remain importable and
must contain only placeholders. This is a path boundary, not a general scanner
for credentials embedded in arbitrary source code.

Git and validation subprocesses receive a toolchain environment allowlist.
Environment filtering does not isolate hostile programs running under the
credential owner's account. Execute untrusted builds under a separate OS
identity or sandboxed worker without keychain access, credential paths, or
unrestricted network access. Do not treat an offline Cargo check as a sandbox.

If a credential was imported before these protections, revoke/rotate it at the
issuer immediately. Stop writers to affected stores and identify snapshots,
sessions, SQLite files, WAL/SHM files, exports, and backups containing the value.
Rebuild a fresh store from reviewed clean sources; verify it before retiring
old databases and backups according to your retention rules. Scriberig never
silently rewrites immutable history. File deletion alone is not a guarantee of
physical erasure on SSDs or replicated backups.

Protected application buffers zeroize on drop, and exact current-credential
reflections are redacted from provider output. HTTP libraries and operating
systems can make additional copies. These measures do not defend against an
OS compromise, a malicious process with equivalent privileges, or arbitrary
encoded disclosure by an untrusted provider.
