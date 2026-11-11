# R11: account-scoped device management

Settings now has a Devices and sessions page with server device IDs/names, an explicit current-device marker, last-seen IP/timestamp, local versus cross-signing trust, and Ed25519 fingerprints when the crypto store has them. Missing crypto metadata is labelled unavailable. The list is bounded to 4,096 devices and 256 visible characters per untrusted field; oversized lists direct the owner to account management rather than silently truncating.

Revocation requires selecting another device and confirming its exact ID. The current device is protected. The SDK sends only that selected ID with automatic POST retries disabled. Success requires a subsequent server list without the target. Network loss, UIA and timeouts remain unconfirmed; they never locally remove a row as proof of revocation. The OAuth account-management entry requests the advertised devices-list action (or its HTTPS account-management base URL), opens without logging the URL, and tells the owner to finish browser authorization and refresh. The application does not collect account passwords or claim to finish a UIA challenge itself.

Requests are owned, capped at 30 seconds, aborted on widget destruction/account cleanup, and fenced by session identity plus nonce. Device verification itself remains the existing SAS/cross-signing/recovery flow from #18/#50. Revoking a token cannot erase data already downloaded on another device.

Validation includes current-device rejection, bounded metadata, real SDK POST target inspection, an authorization challenge without secret leakage, and a stateful HTTP contract fixture whose target token works before revocation and receives 401 afterward while the current token remains usable. This is an isolated contract fixture, not a production homeserver/OAuth integration qualification. Native UI/whole-batch gates are recorded on the PR before leaving draft.

Duplication review: #18 and #50 cover verification/recovery; neither supplies this full session inventory and revocation path. Open PRs #65/#67/#69 address framework pin/form/alert layout and do not duplicate R11. Stacked on R10 #68.

The destructive command carries the session captured by its confirmation. Both dispatch and execution reject a changed session, including a same-user/device re-login; an old target ID cannot be applied under a new account. A fifth SDK-backed identity regression covers this fence.
