# The image a pooled Claude Agent runs in when the owner chooses containers
# for a pool on this computer.
#
# It holds the Claude Code CLI and a few command-line tools, nothing else. It
# has no sign-in: the host passes the pool's credential to each run through
# the environment. It sees none of the owner's files: the host mounts only the
# run's own folders, at the paths they have on the host.
#
# Choosing containers for a pool builds it when the engine does not have it
# yet. The tag is the start of this file's SHA-256, then the owner's user and
# group numbers, so a changed recipe is a new image, and the host runs the
# image by ID: nothing changes for pool Agents until the owner turns
# containers on again.
FROM node:22-bookworm-slim@sha256:e9f565d32859f0cbfb52125eafe8be39390f24dac302dc3cdf566e8ba3ab21ee

ARG CLAUDE_CODE_VERSION=2.1.323

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bubblewrap ca-certificates curl git jq less procps python3 ripgrep socat \
    && rm -rf /var/lib/apt/lists/*

RUN npm install --global --no-fund --no-audit "@anthropic-ai/claude-code@${CLAUDE_CODE_VERSION}" \
    && npm cache clean --force \
    && test -x /usr/local/bin/claude

# A run's folders are the owner's, shared from the host, and a container
# engine on macOS can report their owner as someone else from one moment to
# the next. git refuses to work in a repository it takes for another user's,
# so every repository is trusted here: a run may already write everything it
# can see. A commit needs an author where the repository names none; the one
# a repository names still wins.
RUN git config --system --add safe.directory '*' \
    && git config --system user.name "Hagency Agent" \
    && git config --system user.email "agent@hagency.invalid"

# The host starts /usr/local/bin/claude itself and gives every run its own
# HOME, settings folder and temporary folder.
ENV DISABLE_AUTOUPDATER=1 \
    CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 \
    USER=agent \
    LOGNAME=agent

# The host runs each container as the owner's own user and group, by number.
# The build gives those numbers a name, so that a program asking who it runs
# as (whoami, Node's os.userInfo, Python's getpass) gets an answer.
ARG AGENT_UID
ARG AGENT_GID
RUN if [ -n "${AGENT_UID}" ] && [ -n "${AGENT_GID}" ]; then \
        getent group "${AGENT_GID}" >/dev/null \
            || printf 'agent:x:%s:\n' "${AGENT_GID}" >> /etc/group; \
        getent passwd "${AGENT_UID}" >/dev/null \
            || printf 'agent:x:%s:%s:Hagency Agent:/tmp:/bin/bash\n' "${AGENT_UID}" "${AGENT_GID}" >> /etc/passwd; \
    fi

LABEL org.opencontainers.image.title="hagency-claude-agent" \
      io.hagency.claude-code.version="${CLAUDE_CODE_VERSION}"
