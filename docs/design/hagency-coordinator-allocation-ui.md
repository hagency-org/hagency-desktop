# Coordinator allocation UI checkpoint

An approved project gets an allocation request only when a current owner grant
includes its manager. The coordinator named in that original grant decides the
fixed project amount. Native reservation then runs under the owner's existing
delegation; there is no second human owner approval per project or agent.

The mini app uses closed `hagency.allocations.list/get/decide/release` services.
`project-allocation-review-v1` advertises these new exact read routes; the older
`project-reservations-v1` marker alone does not expose this UI. Both use the
same current reservation/history gate. Read and decision/release grants are separate. No script mapping grants
`ExecuteAllocations` or exposes registry/provider authority. The root-owned native
allocation worker is a separate integration checkpoint.

- `GET allocations/{id}` reads the exact record selected from Inbox or a project.
- `POST projects/{id}/allocations/query` pages visible records before the SQL
  limit, with a 96 KiB serialization budget and a resumable cursor.
- Coordinator decisions use `detail.request.revision`. Release requests use
  `detail.allocation.revision`. They are different after native results.
- Both mutations use the original host-generated private journal command and
  `allocation-commands/{id}/recover`; interrupted replies remain unknown. Only
  audited first-attempt pre-effect failures can be acknowledged as failures.
- Available tokens stay null/Unknown. The total is the immutable owner grant,
  not an estimate of current physical headroom. Native balance and observed
  consumption are separate; rejected cleanup cannot imply a refund.
- History remains accessible for revoked/expired project records when the server
  advertises reservation history. Old servers do not expose unsupported controls.

The server projection exposes existing public allocation balance receipts, not
the private parent receipt, enrollment/device capability or executor bearer.
Current role flags, live grant and CAS are revalidated by server mutations.

Validation: isolated native HTTP/journal tests; real PostgreSQL/HTTP projection,
ACL and pagination tests; actual Splash VM subtree/handler checks. Native full
window captures and a real grant → coordinator decision → owner ledger receipt
flow remain a separate root acceptance step. No running-agent or freed-capacity
claim follows from reservation approval.
