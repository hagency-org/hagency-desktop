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
Codex and Octos resources show no switch, and neither do personal Agents'
usage rows, which are not pools. When the owner turns it on, Desktop, in this
order:

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

**Chosen when the pool is created.** The page that creates a Claude pool has
the same choice, "Run Agents in containers", on computers where Hosting offers
the switch (the session reply says so with `poolContainers`). The package
sends one boolean, `containers`, beside the closed input of
`hagency.accounts.enroll` or `hagency.resources.create`. The host takes it out
before the native command sees its input, refuses it for a pool that is not
Claude, and names it in the native review. Nothing is added to the native
command, so saved requests and older daemons are unaffected.

A pool created this way starts in a container or not at all:

1. Before the creating command runs, its command ID is saved in
   `pool-container-requests.json` in the daemon home. If it cannot be saved,
   or the choices cannot be read or have no room, the pool is not created
   (`pool_containers_unavailable`).
2. When the command's result names the new resource, the resource is added to
   `pools` at once and the command is forgotten. The result can arrive as the
   first reply, as a retry of the same command, or from the saved request
   after a restart (on `hagency.intent.retry`, and whenever the list of saved
   requests is read, which the package does when it opens). The choice holds
   on each of these paths.
3. Turning on then runs in the background, as the switch would run it.

The reply that names the new resource also says what came of the choice, in
`containers`: `preparing` (a container app was found and turning on started),
`no_runtime` (none was found, so its Agents wait) or `unsaved`. `unsaved` means
the resource could not be listed when its first reply arrived, so it would run
in the sandbox; the page then tells the owner to turn containers on in Hosting
before publishing the pool.

Until a runtime is found, the file lists the resource with an empty client,
endpoint and image. A listed resource without a runtime is refused at launch
(`shared_claude_container_not_ready`; a daemon from before this change refuses
it as `shared_claude_container_invalid`), while resources that are not listed
still start in the sandbox. Its page in Hosting shows the switch on, says why
its Agents cannot start, and offers "Try again", which turns on in place. A
turn-on that fails for a resource that is already listed leaves it listed.
Only turning the switch off returns the pool to the sandbox.

**The image.** The Dockerfile is part of the app: Debian with Node 22 (pinned
by digest), Claude Code 2.1.323, and the tools a coding run expects. The tag is
`hagency/claude-agent:<first 12 hex of the Dockerfile's SHA-256>-<uid>-<gid>`
(the numbers of the user who owns the daemon's folders), so a changed recipe,
or another owner account, gets its own image. Runs name the image by ID, never
by tag, and never pull.

The build gives the owner's uid and gid a user and group entry (`agent`) and
sets `USER` and `LOGNAME`, so a program that asks who it runs as (`whoami`,
Node's `os.userInfo`, Python's `getpass`) gets an answer. git trusts every
repository in the container (`safe.directory`): a container engine on macOS
can report a shared folder's owner as another user, and git would then refuse
a repository in the space's project folder. A run may already write everything
it can see, so this opens nothing. git also has a default author, for a
repository that names none; a repository's own author still wins.

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
- **An image stays in use until a turn-on runs again.** Resources turned on
  under an earlier recipe keep its image. Turning any resource on, or creating
  a pool with containers chosen, builds the current image, which then serves
  every resource listed.
- **A pool chosen at creation does not run without a container app.** That is
  the choice: its Agents wait, and the creation note and its Hosting page say
  so. Publishing the pool is not held back.
- **The default commit author is the same for every Agent** (`Hagency Agent`),
  where the repository names none.
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
- The image with a real engine, under the run's isolation settings: in a
  repository at the top of a shared project folder that the engine reported as
  root's, the earlier image failed `git status` ("dubious ownership"),
  `whoami`, `os.userInfo()` and `getpass`, and a commit had no author. The
  current image passes each, and a repository's own author is kept. The
  ignored test that prepares the image passes with the owner's numbers in the
  tag, and the ignored turn test (one turn with a tool command, a local
  stand-in for the model) passes in that image.
- The switch with a real engine (ignored test): two resources turned on one
  after the other, a third left in the sandbox, then each turned off.
- A pool chosen at creation: setting file tests (listed before a runtime is
  found and refused until one is, the creating command kept until its pool is
  known, on each path its result can take), host tests (one boolean beside the
  closed input, Claude pools only), the app's hosting test (a failed turn-on
  leaves a listed resource listed), and Hosting screen model and widget tests
  (the waiting state, its texts and "Try again"). The creation page in the
  real widgets, both languages: the switch and its note appear only for a
  Claude pool where the session offers them. The ignored real-engine test of
  the switch also creates a pool this way: listed at once, never the sandbox,
  turned on without a switch being pressed.
- Setting file tests in the `hagency` crate: each resource's choice, one
  runtime for all, the last one off removing the file, and files that cannot
  run. Hosting screen model and widget tests: the switch only on a Claude
  pool's page and never on a personal Agent's row, its states and texts. The
  app's hosting merge test.
- A live run through the owner daemon with Docker Desktop, made with the first
  version of the switch, which applied to every pooled Claude resource on the
  computer. The owner turned the switch on in Hosting, and a member's pooled
  Claude Agent then ran in its own container: the runtime reported Linux, and
  the room's history tool answered through the container's stdio. The
  container held only the run's scope, under the settings above, and was
  removed when each run ended. After the switch was turned off, the next run
  used the macOS sandbox again.
- A live run of this version through the owner daemon with Docker Desktop. The
  owner turned the switch on for that Claude resource in Hosting, and the
  saved choice listed only that resource. A member's Agent from that pool then
  ran in its own container (the runtime reported Linux), and the container was
  removed when the run ended. A new Desktop window showed the switch on. After
  the owner turned it off, the file was gone and the next run used the macOS
  sandbox (Darwin). The Codex and Octos resources' pages showed no switch.

## Not covered

Linux and Windows owner computers, pooled Codex and Octos, local Agents,
memory and CPU limits, and a network policy for the container.
