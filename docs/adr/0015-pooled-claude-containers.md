# ADR 0015: Pooled Claude Agents in a container

- Date: 2026-11-18
- Status: Accepted; implemented for pooled Claude Agents behind an opt-in
  switch for each pool in Hosting. Checked on macOS with Docker Desktop. Linux
  hosts, Codex and Octos pools, and local Agents are not covered.
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

An owner can choose, for each pool, to run its Claude Agents in a container.
The macOS sandbox stays the default. Local Agents, pooled Codex and pooled
Octos are unchanged.

**The switch.** In Hosting, the page of each Claude resource (a pool's
resource on this computer) shows "Run this resource's Agents in containers".
Codex and Octos resources show no switch. When the owner turns it on, Desktop,
in this order:

1. finds a Docker-compatible client in the usual install places;
2. reads its current context's endpoint, accepting only a local `unix://`
   socket;
3. builds the image if the engine does not have it;
4. adds the resource to the saved choice.

If a step fails, the switch says what to do: install a container app, start
it, or try again after a failed build. Desktop makes one change at a time.

The choice is `pool-containers.json` in the owner daemon's home:
`{cli, endpoint, image, pools}`. `image` is an image ID, and `pools` lists the
resource IDs whose Agents run in containers. The client, endpoint and image are
this computer's runtime: turning any resource on finds them again, and they
then serve every resource listed. The daemon reads the file at each pooled
Claude launch, so a change applies from the next run, and it uses a container
only when the launch's resource is listed. Other resources keep the macOS
sandbox, and with no file every resource does. Turning the last resource off
removes the file. A lock file next to it keeps two Desktop windows from
dropping each other's changes.

A file that cannot run is never ignored. If it cannot be read, it cannot say
which resources chose containers, so every pooled Claude launch is refused
(`shared_claude_container_invalid`), and every Claude resource's switch says
its Agents cannot start. Turning any of them off removes the file, which
returns every resource to the sandbox. A listed resource whose client is gone
is refused the same way, while the others still start in the sandbox.

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
- Credentials are passed as `--env NAME`, so their values are never on a
  command line. The engine keeps them in the container's configuration while
  the container exists; see Consequences.
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
- **The engine holds the run's credential while the container exists.** The
  client sends the values it was given by name to the engine, which keeps them
  in the container's configuration. Anyone who can use the engine (on a Mac,
  the owner's own account) can read them with `docker inspect` until the run
  ends and the container is removed. No engine socket is mounted into the
  container. Docker Desktop's optional TCP endpoint without TLS (off by
  default) would be reachable from it through `host.docker.internal`, so an
  owner who uses containers must leave that endpoint off.
- **The container reaches the host's network.** `host.docker.internal` leads
  to services that listen on the Mac's loopback, as any process on the Mac
  would.
- **The choice is kept on the computer.** It lives in this computer's daemon
  home, so a pool handed over to another computer runs there in the sandbox
  until its owner turns containers on there.
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
- The switch with a real engine (ignored test): two resources turned on one
  after the other, a third left in the sandbox, then each turned off.
- Setting file tests in the `hagency` crate: each resource's choice, one
  runtime for all, the last one off removing the file, and files that cannot
  run. Hosting screen model and widget tests: the switch only on a Claude
  resource's page, its states and texts. The app's hosting merge test.
- A live run through the owner daemon with Docker Desktop, made with the first
  version of the switch, which applied to every pooled Claude resource on the
  computer. The owner turned the switch on in Hosting, and a member's pooled
  Claude Agent then ran in its own container: the runtime reported Linux, and
  the room's history tool answered through the container's stdio. The
  container held only the run's scope, under the settings above, and was
  removed when each run ended. After the switch was turned off, the next run
  used the macOS sandbox again.
- This version's switch against the real owner daemon, read only: the page of
  that Claude resource shows it, off, and the Codex and Octos resources' pages
  show none.

## Not covered

Linux and Windows owner computers, pooled Codex and Octos, local Agents,
memory and CPU limits, and a network policy for the container.
