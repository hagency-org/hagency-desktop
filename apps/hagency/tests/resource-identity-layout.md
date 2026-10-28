# Resource identity and Chinese layout review

The starting evidence is genuine full-window native captures:

- `.run/management-resource-detail-e2e-13-cn/captures/resource-list-return-zh-CN.png`: two model/budget-identical cards; list UX 8.28.
- `.run/management-project-setup-e2e-12/captures/coordinator-read-only-cn-dark.png`: project name dominated the setup task; UX 8.53.
- `.run/management-integration-project-e2e-12b/captures/coordinator-resource-connection-cn-dark.png`: current contextual connection, with Back and a labelled project row. The older contextual checkpoint scored 8.93; it is not a score for this revision.

Changes use the existing shared theme and actual native data:

1. Model/framework is the resource title, with a compact ID beneath it. The full native ID appears in detail. Short-ID collisions on a page expand to full IDs. No invented resource names or extra service capability.
2. Available/limit, period, reserved, observed usage and capabilities share aligned label/value rows. The common native-ledger/provider-limit caveat appears once above the list. Resource and connection actions share one wrapping row.
3. Setup uses the task as its title and a labelled project context. The current prerequisite blocker uses normal body text. A theme divider separates prerequisites from Matrix-room progress; the explicit creation and verification rows remain.
4. Connection detail presents resource name before project context and separates people/resource metadata from the current check and next action. Every prior role gate and native callback is preserved.
5. Conditional named controls are scoped in real containers to avoid the pinned Makepad VM's skipped-child corruption. No mutation/recovery handler was changed.

Validation output is under `.run/resource-identity-layout/final`: 14 resource-list,
12 resource-detail, 12 room-progress, 36 agent-admission and 16 complete
setup/connection subtree checks passed in English/Chinese. Package/catalog tests
passed 8/8; i18n reports no missing entries. The actual Makepad parser passed.
These are script-object and route checks, not pixel/layout acceptance.

Root's frozen admission release14 and its source overlay are unchanged. This is
a separate checkpoint. It has not been launched or scored. Native acceptance
must still verify long model/ID/project wrapping, complete button bounds,
EN/light and CN/dark rendering, read-only coordinator actions, original command
retention, and the same resource ID after Refresh/Back. No new 9/10 claim is made.
