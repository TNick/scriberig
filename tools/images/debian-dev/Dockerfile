# Generic Debian 13 development image with common language toolchains and test tools.
FROM node:24-trixie AS node

FROM python:3.14-slim-trixie

ARG PNPM_VERSION=12.11.2
ARG PLAYWRIGHT_VERSION=1.64.0

ENV DEBIAN_FRONTEND=noninteractive \
    PATH="/root/.cargo/bin:${PATH}" \
    PLAYWRIGHT_BROWSERS_PATH=/opt/ms-playwright

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        cmake \
        curl \
        file \
        git \
        golang-go \
        jq \
        libffi-dev \
        libssl-dev \
        libsqlite3-dev \
        make \
        perl \
        php-cli \
        php-curl \
        php-mbstring \
        phpunit \
        php-xml \
        php-zip \
        pkg-config \
        shellcheck \
        unzip \
        xz-utils \
        yamllint \
        zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/bin/corepack /usr/local/bin/corepack
COPY --from=node /usr/local/bin/npm /usr/local/bin/npm
COPY --from=node /usr/local/bin/npx /usr/local/bin/npx
COPY --from=node /usr/local/lib/node_modules/corepack /usr/local/lib/node_modules/corepack
COPY --from=node /usr/local/lib/node_modules/npm /usr/local/lib/node_modules/npm

RUN corepack enable \
    && corepack prepare "pnpm@${PNPM_VERSION}" --activate \
    && npm install --global "@playwright/test@${PLAYWRIGHT_VERSION}" \
    && python -m pip install --no-cache-dir "mkdocs-material>=9,<10" mypy pre-commit pytest pytest-cov pytest-xdist ruff uv \
    && rustup_installer="$(mktemp)" \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$rustup_installer" \
    && sh "$rustup_installer" -y --profile minimal --default-toolchain stable --component rustfmt --component clippy \
    && rm -f "$rustup_installer" \
    && mkdir -p /opt/ms-playwright \
    && playwright install --with-deps chrome \
    && rm -rf /var/lib/apt/lists/* /root/.cache/pip

WORKDIR /workspace
