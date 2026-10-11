# Licensing Scriberig

Scriberig intentionally uses two licenses that follow the architectural boundary between reusable platform components and the integrated product. This guide explains that boundary; it does not replace or override the applicable license text or package metadata.

## Overview

- Reusable platform and building blocks: **Apache-2.0**.
- Integrated Scriberig product and runtime: **AGPL-3.0-only**.

Architectural responsibility determines which side a component belongs to. The programming language, package type, or use of a particular UI framework does not determine the license by itself.

## Apache-2.0 reusable platform

Reusable infrastructure is Apache-2.0 by default. Representative components include:

- core identity and value types;
- storage abstractions and reusable storage backends;
- graph and schema libraries;
- query and migration/change libraries;
- projection infrastructure;
- domain adapters;
- protocol definitions;
- Rust and TypeScript clients;
- client-side semantic models and framework bindings;
- headless editor/navigation behavior;
- neutral reusable UI components;
- reusable MUI component libraries;
- reusable agent-facing interfaces that do not implement the integrated Scriberig runtime.

The integrated Scriberig product may depend on these libraries without changing their Apache-2.0 classification. Third parties may also build applications with Apache-licensed Scriberig components subject to the terms of the Apache License 2.0.

## AGPL-3.0 integrated product

The integrated Scriberig product is AGPL-3.0-only by default. Representative components include:

- the integrated/reference Scriberig server product;
- integrated agent execution and orchestration;
- the native product composition executable when it embeds the integrated runtime;
- the complete Scriberig browser editor application;
- product-specific workflows whose purpose is to operate the complete Scriberig product.

Not every server-related library, binary, or agent-related abstraction is automatically AGPL. Reusable protocol, client, UI, and agent-interface libraries remain on the Apache side when their architectural role is reusable infrastructure.

## How to determine a component's license

Each distributable Cargo or npm package declares its own SPDX license identifier in its package metadata. That declaration is the immediate indicator for that component.

The repository intentionally contains both Apache-2.0 and AGPL-3.0-only packages. Do not infer one repository-wide license from the license of a neighboring package.

The expected dependency direction is:

```text
Apache reusable platform
        ^
        |
AGPL integrated product
```

AGPL product components may depend on Apache platform components. Reusable Apache components must not depend on integrated AGPL product components.

## Agent and CLI boundary

Reusable agent contracts, tool descriptions, semantic navigation interfaces, and similar building blocks may be Apache-2.0. The integrated agent execution/orchestration runtime is AGPL-3.0-only.

Likewise, a CLI that primarily acts as a reusable client over public APIs may be Apache-2.0. A composition executable that embeds the integrated Scriberig runtime follows the AGPL product license.

## Third-party applications and extensions

An independent application does not become part of the integrated Scriberig AGPL product merely because it uses Apache-licensed Scriberig libraries or interoperates through Scriberig's public protocol.

The license consequences of a concrete combined or derivative work can depend on how software is actually combined and distributed. This document describes Scriberig's architectural licensing intent rather than providing a legal opinion for every possible integration.

## Third-party dependencies

Third-party dependencies, assets, generated code, fonts, parsers, and other external material retain their own licenses. Required attribution and notices must be preserved when those materials are distributed.

Repository-level notices are recorded in `THIRD_PARTY_NOTICES.md` when applicable. That file does not replace the licenses of individual dependencies.

## Source headers and SPDX metadata

Package-level license metadata is required for distributable first-party packages. The current repository policy does not require a license or SPDX header in every source file.

A future source-header policy may be adopted separately, but it must not silently change the license assigned to a package.

## Contributions and future dual licensing

Contributions follow the license of the component being modified. See `CONTRIBUTING.md` for contributor-facing guidance.

The repository currently does not imply a CLA, DCO, copyright assignment, or general commercial-relicensing grant. Commercial dual licensing of AGPL-covered components may be considered in the future, but it is not promised or implicitly authorized by the current contribution terms.

## License texts

The complete selected license texts are available here:

- [Apache License 2.0](LICENSES/Apache-2.0.txt)
- [GNU Affero General Public License version 3](LICENSES/AGPL-3.0.txt)

Those license texts and the license declaration of the applicable component govern. This guide is explanatory documentation only.
