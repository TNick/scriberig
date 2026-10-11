# Licensing

Scriberig uses an architectural licensing split.

## Reusable platform

Reusable infrastructure is Apache-2.0 by default. This includes core identities, storage abstractions/backends, schemas, graph/query/migration/projection libraries, domain adapters, protocol/client libraries, reusable agent interfaces, and reusable TypeScript/React/MUI packages.

## Integrated product

The integrated Scriberig product is AGPL-3.0 by default. This includes the integrated agent runtime/orchestrator, reference server product, native composition executable, and complete browser editor application.

## Package authority

Each distributable Cargo or npm package declares its own SPDX license identifier. That package metadata is authoritative for the component. The architectural rule is that AGPL product components may depend on Apache platform components; reusable Apache components must not depend on integrated AGPL components.

The `LICENSES/` directory is reserved for complete authoritative license texts. The scaffold intentionally does not fabricate or shorten legal text; the dedicated licensing implementation task must populate the selected complete license texts before distribution.

Third-party dependency licenses remain applicable independently of Scriberig's own licenses. See `THIRD_PARTY_NOTICES.md` for repository-level notices when required.
