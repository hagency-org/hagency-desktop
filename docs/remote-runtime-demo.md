# Remote Agent runtime demo

Hagency Desktop can use an independently operated
[Hagency Agent Runtime Server](https://github.com/chrislearn/hagency-agent-runtime-server)
for an owned Agent. The Desktop owner daemon still handles the Agent identity,
device authorization and Matrix reply. The runtime host runs its installed
Codex or Claude CLI using the credentials it controls.

## Bind a runtime

1. Sign in to Hagency Desktop and enable Hosting for your account on the
   device assigned to the Agent. Create an owned Agent under **My Agents**
   and open its private chat. This flow needs no resource pool.
2. Ask the runtime operator to issue a grant for your Matrix user ID, this
   Agent's ID and the provider/model. If the Agent already has a selected
   model, the remote provider and model must match it. An Agent without a saved model can
   use the model named in its invitation.
3. On the execution device, stop the Agent service. Find the owner daemon's
   `owned-agent-owners/owner_<digest>/` directory for this account. Its
   `profile-identity.json` gives the owner and authority, alongside
   `agent-local.sqlite`. On macOS, look under
   `~/Library/Application Support/Hagency/owner-daemon/`. On Linux, look
   under `${XDG_STATE_HOME:-$HOME/.local/state}/hagency/owner-daemon/`.
4. Inside that account directory create `runtime-bindings/` with mode 700.
   Save the invitation as `runtime-bindings/<agent-id>.json` with mode 600.
   Keep the token on the execution device and use the operator's HTTPS
   origin.

```json
{
  "endpoint": "https://runtime.example",
  "binding_id": "<binding UUID from grant>",
  "token": "<binding token from grant>",
  "provider": "codex",
  "model": "<model from grant>"
}
```

Refresh the Agent view to see its runtime origin. Start its service and send a
message in the unencrypted private chat. Leave shared workspace access off.
The demo forwards the current message only and does not send conversation
history, files, tool credentials or decrypted messages. Runtime status and
the original conversation remain available from the Agent. Local execution
options and budgets apply to local runs. The runtime operator controls its
own eligibility and provider account.

## Stop, switch or reconcile

Stop the service before changing or removing the invitation. Save the new
invitation and start the same Agent. New runs use the new binding. This works
while the previous runtime is unavailable. The Agent ID and Matrix room
bindings stay in place.

Desktop writes a private receipt under `remote-runtime-runs/<agent-id>/` before
submission. The Agent view shows unresolved runs. The receipt records the
original run ID, binding ID and endpoint.
Keep the previous invitation securely until earlier runs are resolved.
For a run pending reconciliation, query `GET /v1/runs/<run-id>` on the
recorded endpoint with the original binding token. After confirming its outcome,
archive the receipt outside that folder. If the service is stopped
during execution, Desktop requests cancellation from that same binding.
An interrupted request can be retried only through its original binding
and run ID.

The demo covers owner-direct text. User, room and project sharing, encrypted
remote execution, reservations, and migration of resource-pool obligations
remain rollout work under [issue #238](https://github.com/hagency-org/hagency-desktop/issues/238).
