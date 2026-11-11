# R07: private attachment share lifecycle

This adapts the already-merged Client 19/20/21 implementation to Desktop's official Matrix SDK and current session/timeline APIs. It stacks on R06/59 (which stacks on R05/58 and R03/57). Existing Desktop 53 does not cover private sharing. No SDK upgrade is included.

## Authority and storage

A single tracked filesystem worker stages verified R05 file handles with streaming copies, not full-file byte buffers. Each staged file has an unpredictable private run/job directory, owner-only permissions (Unix 0700/0600; protected Windows owner DACL through hagency-store), retained directory handles and original physical file identity. NoFollow traversal refuses symlinks/reparse subtrees and replacements. Native path aliases are checked before preparation. An original cross-process lease protects other live instances during crash recovery.

Share scopes bind both account epoch and exact SDK revision. Account close revokes the closing epoch before incrementing it; queued/running IO rechecks its scope. Preparation runs outside the UI. Actual native presentation and Windows delayed payload delivery recheck a live ticket under the short session IO fence, including reentrant-call rejection. UI results from retired accounts cannot authorize later presentation.

The worker admits at most eight queued/running jobs and 32 staged/native operations. File sharing has no new 256 MiB cutoff: it participates in the common 16 GiB temporary-storage policy and the same logical envelope as download pipelines. Actual file ownership and reserved future pipeline headroom are separately charged. Both reservations follow the original share ticket, including native delayed payload ownership after confirmed pathname cleanup. Failed cleanup retains its reservation; unknown partial-file cleanup poisons further staging pending crash recovery. A 32-bit build explicitly rejects sizes that cannot be represented by its address-size accounting.

## Cleanup and native outcomes

Presented files expire after 30 minutes, checked by minute sweeps; account retirement and application shutdown request earlier cleanup. Known presentation failure discards immediately. Locked live runs survive crash sweeping. Unlocked crashed runs are recovered at startup; periodic sweeps also retry incomplete runs without a lease after their safety TTL. Legacy cleanup is confined to application-managed cache/temp, without following symlinks. Ordinary user Save As destinations are not staging targets. Crypto databases, room keys, event attachment descriptors and server ciphertext are not deleted by this cleanup. Cache removal remains recoverable only while the server media and required event/key material are available.

A vendored MIT robius-share patch separates blocking preparation from guarded presentation and returns an owned PendingShare. Linux portal sends are nonblocking and observed on the original worker, with bounded control responses/deadlines; fallback is allowed only before any portal Open request. Partial sends/lost replies are unknown, never justification for a second launch. Shutdown observes its original launcher/socket; timeout or cleanup failure is logged as incomplete, never successful deletion.

Presentation is not successful delivery or known cancellation, so the spinner returns to idle without a misleading Shared verdict. There is no native completion callback: TTL expiration can make a recipient that waits beyond 30 minutes unable to open the local source. Already handed-off descriptors or copies held by external applications cannot be revoked or erased by this process.

## Validation boundary

Regression tests exercise real private files/permissions, physical identity replacement, original-directory cleanup retries, crash/live-process leases, periodic missing-lease recovery, TTL/account/shutdown cleanup, retirement during IO, native authority after unlink, admission and coupled quota rollback. The native bridge tests veto actual presentation without sending to a recipient; presentation fences and UI outcomes have separate tests. Required release gates include these selectors and the R05 large-file/integrity fixtures. Detailed counts are recorded in the PR description. macOS is the local build/test host; Windows/Linux native UI and real recipient delivery are not certified by those tests. Existing unrelated warning-as-error CI failures are not suppressed here.
