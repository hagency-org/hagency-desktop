# Required field labels

`mod.widgets.RequiredFieldLabel` provides a caption and a theme danger-colored
asterisk. The corresponding input placeholder and accessible name include
the localized required meaning (`Give it a name (required)`). The room/space wizard shares one details form across ordinary spaces,
Agent spaces, and chat rooms. Empty names show a hint after editing, including whitespace-only input
and after clearing it. A freshly opened form shows the label and star without an error.

## Other forms to adopt this treatment (inventory only)

- `src/login/login_screen.rs`: homeserver, password sign-in, registration fields.
- `src/home/invite_modal.rs`: Matrix user ID when inviting someone.
- `src/home/space_management.rs`: name when editing an existing space.
- `src/tsp/create_wallet_modal.rs` (feature gated): wallet name and password fields.
- `src/tsp/create_did_modal.rs` (feature gated): identity username.
- `apps/hagency/bundle/main.splash`: Agent/model/policy/resource configuration forms
  with mandatory names or credentials; each should identify its mandatory fields.

## Verification

In English and Chinese, open each of the three creation paths in light and dark
mode. The input accessible name says required; the asterisk uses
the danger role. Initially no empty-name hint is shown. Enter a valid name and clear it again,
then enter only whitespace. Empty names after editing show a short hint and disable Create. Check the hint returns and the optional
description stays unchanged. Room creation still requires a known writable space;
Agent spaces keep their existing private settings. Do not create live resources
just to inspect these states.
