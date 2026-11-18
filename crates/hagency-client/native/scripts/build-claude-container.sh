#!/bin/sh
# Builds the image pooled Claude Agents run in when containers are on for
# pool Agents, with the tag the host looks for. Turning containers on in the
# Hosting screen does the same when the image is missing. Works with any
# Docker-compatible client: set DOCKER to its path if it is not `docker`.
#
#   scripts/build-claude-container.sh
#
# Prints the image ID.
set -eu
docker="${DOCKER:-docker}"
here="$(cd "$(dirname "$0")/.." && pwd)"
recipe="$here/hagency-runtime/src/owned/claude_shared/container/Dockerfile"
digest="$(shasum -a 256 "$recipe" | cut -c1-12)"
tag="hagency/claude-agent:${digest}"
"$docker" build --tag "$tag" - < "$recipe" >&2
"$docker" image inspect --format '{{.Id}}' "$tag"
