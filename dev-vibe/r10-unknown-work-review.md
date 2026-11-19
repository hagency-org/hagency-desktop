# R10: review unknown execution without replay

An unknown run may already have changed external systems or incurred costs. The notice now says so and opens a read-only history review tied to the original session, Agent, room and Matrix event. It never sends another prompt, restarts work, releases usage, or treats missing history as proof of zero cost.

History pages are capped at 16 records. Before reading a selected request, the native management bridge verifies its assignment and room membership against the original Agent and room. Only `ReadAgentWork` and `ReadAgentStatus` permissions are requested. Account retirement, closing and navigation cancel the owned request; late callbacks must match the original session and nonce. Known consumption and retained accounting are displayed separately without adding them together.

Validation: six `work_review` tests cover exact read commands, bounded response parsing, request/Agent/room/assignment identity, unknown-side-effect accounting, read-only controls, Back and account cleanup. The permission-notice selector and translation completeness are checked separately. These fixtures do not claim a live tool/model execution or an end-to-end server settlement test. Full batch gates remain pending while this PR is draft.

PR duplication review: open and historical Desktop PRs include #22 (pool work history), #50 (compatible Robrix changes), and #61 (permission recovery). They do not cover the unsafe unknown-run resend guidance or this original-message review. This PR is stacked on R09 #66; its diff contains only R10.
