#!/bin/sh
# Builds the image pooled Claude Agents run in when containers are chosen for
# a pool, with the tag the host looks for. Choosing containers in the app does
# the same when the image is missing. Works with any Docker-compatible
# client: set DOCKER to its path if it is not `docker`.
#
#   scripts/build-claude-container.sh
#
# Run it as the user who hosts the pools: the image names that user, whose
# user and group numbers end the tag. Prints the image ID.
set -eu
docker="${DOCKER:-docker}"
here="$(cd "$(dirname "$0")/.." && pwd)"
recipe="$here/hagency-runtime/src/owned/claude_shared/container/Dockerfile"
digest="$(shasum -a 256 "$recipe" | cut -c1-12)"
uid="$(id -u)"
gid="$(id -g)"
tag="hagency/claude-agent:${digest}-${uid}-${gid}"
"$docker" build --build-arg "AGENT_UID=${uid}" --build-arg "AGENT_GID=${gid}" --tag "$tag" - < "$recipe" >&2
"$docker" image inspect --format '{{.Id}}' "$tag"
