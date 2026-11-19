# The image a pooled Claude Agent runs in when the owner turns on containers
# for pool Agents on this computer.
#
# It holds the Claude Code CLI and a few command-line tools, nothing else. It
# has no sign-in: the host passes the pool's credential to each run through
# the environment. It sees none of the owner's files: the host mounts only the
# run's own folders, at the paths they have on the host.
#
# Turning containers on in the Hosting screen builds it when the engine does
# not have it yet; scripts/build-claude-container.sh builds it by hand. The
# tag ends with the start of this file's SHA-256, so a changed recipe is a new
# image, and the host runs the image by ID: nothing changes for pool Agents
# until the owner turns containers on again.
FROM node:22-bookworm-slim@sha256:e9f565d32859f0cbfb52125eafe8be39390f24dac302dc3cdf566e8ba3ab21ee

ARG CLAUDE_CODE_VERSION=2.1.323

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bubblewrap ca-certificates curl git jq less procps python3 ripgrep socat \
    && rm -rf /var/lib/apt/lists/*

RUN npm install --global --no-fund --no-audit "@anthropic-ai/claude-code@${CLAUDE_CODE_VERSION}" \
    && npm cache clean --force \
    && test -x /usr/local/bin/claude

# The host starts /usr/local/bin/claude itself and gives every run its own
# HOME, settings folder and temporary folder.
ENV DISABLE_AUTOUPDATER=1 \
    CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1

LABEL org.opencontainers.image.title="hagency-claude-agent" \
      io.hagency.claude-code.version="${CLAUDE_CODE_VERSION}"
