# ADR 0015: Pooled Claude Agents in a container

- Date: 2026-11-18
- Status: Accepted; implemented for pooled Claude Agents behind an opt-in
  Hosting switch. Checked on macOS with Docker Desktop. Linux hosts, Codex and
  Octos pools, and local Agents are not covered.
- Scope: Hagency Desktop and its embedded Hagency Client. No server change.

## Context

A pooled Agent runs a coding CLI on the owner's computer for people who rent
it. Today a pooled Claude run is confined by the macOS sandbox: a
`sandbox-exec` profile that limits the CLI to the run's own folders
(`host-owned-room-v1`). The CLI's tool commands then run under Claude Code's
own sandbox. Both layers run on the owner's own kernel and user account.

Some owners want a stronger wall between a renter's commands and their own
computer. Docker Desktop, OrbStack, Colima, Podman and other engines that speak
the Docker API run Linux containers in a virtual machine. A container also gives
the run a fixed set of programs instead of the owner's.

## Decision

An owner can choose, per computer, to run pooled Claude Agents in a container.
The macOS sandbox stays the default. Local Agents, pooled Codex and pooled
Octos are unchanged.

**The switch.** Hosting shows "Run rented Claude Agents in containers". When
the owner turns it on, Desktop, in this order:

1. finds a Docker-compatible client in the usual install places;
2. reads its current context's endpoint, accepting only a local `unix://`
   socket;
3. builds the image if the engine does not have it;
4. saves the choice.

If a step fails, the switch says what to do: install a container app, start
it, or try again after a failed build.

The choice is `pool-containers.json` in the owner daemon's home:
`{cli, endpoint, image}`, where `image` is an image ID. The daemon reads it at
each pooled Claude launch, so a change applies from the next run. With no file,
runs use the macOS sandbox. A file that cannot run refuses the launch
(`shared_claude_container_invalid`); it is never ignored.

**The image.** The Dockerfile is part of the app: Debian with Node 22 (pinned
by digest), Claude Code 2.1.323, and the tools a coding run expects. The tag is
`hagency/claude-agent:<first 12 hex of the Dockerfile's SHA-256>`, so a changed
recipe gets a new tag. Runs name the image by ID, never by tag, and never pull.

**The run.** Each launch is one `docker run --rm --interactive --init` with the
CLI's normal arguments. Claude talks to the host over stdin and stdout as it
does natively.

- Only the run's own folders are mounted, at the same paths as on the host:
  the room's scope, plus the space's project folder when there is one. The root
  filesystem is read-only, and `/tmp` is a private tmpfs that is also the CLI's
  temporary folder.
- The container runs as the owner's uid and gid, never as root, with every
  capability dropped, `no-new-privileges`, and at most 1024 processes.
- Credentials are passed as `--env NAME`. Their values are only in the client's
  environment, never on its command line.
  `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1` keeps them away from tool commands.
- The client runs with an empty settings folder of its own in the daemon home,
  outside every run's folders, and a fixed `PATH`. No `DOCKER_*` or
  `BUILDKIT_*` value can come from a run's environment.
- A model endpoint on the host's loopback is refused, because inside the
  container that address is the container itself.

**Cleanup.** Every container has a unique name and two labels: the role and a
digest of this host's client folder. Stopping a run removes its container, and
so does a failed start. The first launch in each daemon process removes any
containers with this host's label left by an earlier process. It holds a lock
so that two processes on one daemon home never sweep each other's runs. If a
removal fails, the run's cleanup is reported as unknown, not as done.

**Sessions.** The session settings digest names the isolation:
`host-owned-room-v1` for the sandbox, or `container-room-v1:<image ID>`. A
saved Claude session is kept under a scope that includes this digest. So after
a change of mode or image, a member's next run starts a fresh session instead
of resuming one made under the other.

## Consequences

- **bubblewrap needs seccomp and `/proc` masking off.** Claude Code's tool
  sandbox on Linux is bubblewrap, and it needs `seccomp=unconfined` and
  `systempaths=unconfined` to create its namespaces in the container. That
  widens the kernel surface open to a renter's commands, toward the engine's
  virtual machine. Docker Desktop's machine shares `/Users`, so an escape from
  the container into that machine would reach the owner's files. A seccomp
  profile that allows only what bubblewrap needs is the follow-up.
- **The container reaches the host's network.** `host.docker.internal` leads
  to services that listen on the Mac's loopback, as any process on the Mac
  would.
- **The first switch-on takes minutes and needs the network**, to download
  the base image and Claude Code. Later switch-ons find the image at once.
- **Claude Code's version is the image's.** A new version is a new Dockerfile
  and so a new tag, built on the next switch-on.
- **Start failures** reach the caller as `shared_claude_container_unavailable`
  (409), and the daemon logs `shared_claude_container_start_failed`.

## Checks

- Runtime unit tests for the command line, the refusals, mount quoting, the
  sweep, cleanup after a failed start, and bounded client commands:
  `makepad test --allow-c hagency-runtime --lib -- container`.
- A turn with a real engine (ignored test): a model fixture asks for a Bash
  tool, which runs in the container. The test checks that the command sees no
  credential and not the CLI's process, cannot write the image, and works in
  the run's folder. The containers are gone after a normal end, after the
  client is killed, and after another host's sweep.
- The switch's setup step with a real engine (ignored test).
- Setting file tests in the `hagency` crate, Hosting screen model and widget
  tests, and the app's hosting merge test.

## Not covered

Linux and Windows owner computers, pooled Codex and Octos, local Agents,
memory and CPU limits, and a network policy for the container.
