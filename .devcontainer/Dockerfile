FROM ghcr.io/prefix-dev/pixi:0.81.0

# The official Pixi image is intentionally small; add only host tools needed for
# Git-based development and VS Code's container integration.
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends \
        ca-certificates \
        git \
        openssh-client \
    && rm -rf /var/lib/apt/lists/*
