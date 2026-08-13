# syntax=docker/dockerfile:1
# Emergency / sidecar image for ChefBar Cloud Agent work (Daytona nood-runner,
# local Linux boxes). Not the Cursor Cloud base: Cloud Agents keep the default
# Cursor image and bootstrap via `.cursor/install.sh` + `.cursor/start.sh`.
#
# Do not COPY the ChefBar source into this image. Cursor / Daytona check out
# the requested git revision separately.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    BUN_INSTALL=/usr/local/bun \
    PATH=/usr/local/bun/bin:/usr/local/cargo/bin:/usr/local/bin:${PATH}

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        git \
        pkg-config \
        python3 \
        python3-venv \
        unzip \
        libgtk-3-dev \
        libglib2.0-dev \
        libcairo2-dev \
        libpango1.0-dev \
        libgdk-pixbuf-2.0-dev \
        libatk1.0-dev \
        libnss3 \
        libnspr4 \
        libatk-bridge2.0-0t64 \
        libcups2t64 \
        libdrm2 \
        libxkbcommon0 \
        libxcomposite1 \
        libxdamage1 \
        libxfixes3 \
        libxrandr2 \
        libgbm1 \
        libasound2t64 \
        libxshmfence1 \
        fonts-liberation \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --default-toolchain stable --profile minimal \
    && rustup default stable

RUN curl -fsSL https://bun.sh/install | bash

WORKDIR /workspace
CMD ["bash"]
