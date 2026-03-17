# syntax=docker/dockerfile:1
FROM debian:bookworm-slim

# Install core dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    gnupg \
    build-essential \
    git \
    zip \
    unzip \
    nano \
    tree \
    tmux \
    htop \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Install gitleaks
RUN curl -sSfL https://github.com/gitleaks/gitleaks/releases/download/v8.30.0/gitleaks_8.30.0_linux_x64.tar.gz \
    | tar -xz -C /usr/local/bin gitleaks \
    && chmod +x /usr/local/bin/gitleaks

# Install opencode
RUN curl -fsSL https://opencode.ai/install | bash

# Configure git defaults
RUN git config --global init.defaultBranch main \
    && git config --global user.email "agent@local" \
    && git config --global user.name "Agent"

# Workspace directory (will be bind-mounted)
WORKDIR /workspace

# Keep container running
CMD ["tail", "-f", "/dev/null"]
