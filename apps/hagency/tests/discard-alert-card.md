# Alert card review (#63)

## Code findings

`HagencyAlert` draws a rounded background, but a `RoundedView` does not mask its
children to that shape. Its children are `height:Fit` containers containing the
title, message, dividers and buttons, so there is no evidence that the Fit card
height excludes the action rows. The definite geometry defect is that the bottom
actions draw independent square faces (`border_radius:0`).

The actions inherit `RinxKeyButton` → `RinxButton` → `Button` and its background
shader. That shader has secondary gradient stops, bevel strokes, material/state
layers and focus/hover/down mixes. The old alert overrides only some of these
inputs. `RinxButton` does disable the resting gradient with a negative sentinel,
so the presence of a gradient in a platform theme alone does not prove the gray
resting rows' cause. Native before/after captures must confirm that symptom;
these source/VM checks do not establish its pixel-level origin.

The alert actions now own a minimal flat shader: transparent at rest and focused,
only theme hover/down fills, no bevel, gradient or material pass. This removes
inherited painting paths from these rows. Explicit bottom corner ownership masks
the action fill itself: both corners for the final vertical action; left and
right outer corners for a horizontal pair. Hairline dividers and callbacks stay
unchanged. Sdf2d doubles radii, so card and actions both use 7 for a 14pt corner.
Danger text also retains its danger role when keyboard focused.

## Native verifier handoff

Use an isolated data directory. Capture the baseline `fa58b7f8` first and this
branch afterwards; do not use the owner's installed data or submit any edits.

For each of `model`, `policy`, `pool` and `encrypt`, in light and dark mode:

1. Open the alert from the corresponding unsaved-edit or encryption confirmation
   flow. Move the pointer away, and capture the entire card at rest.
2. Hover each action, then hold its press and capture it. Inspect both lower
   corners, including the pool/encryption horizontal action pairs: highlights
   must end at the card's curve, with no separate gray block or square protrusion.
3. Check a uniform card surface, hairline dividers, danger discard text and bold
   Keep Editing. Tab through actions and check text color/focus behavior.
4. Cancel/Keep Editing and ensure the draft remains. Do not activate destructive
   confirmation or enable encryption merely to inspect appearance.

Attach the actual before/after captures to the issue/fix. No screenshots were
created by the source-only worker; pixel acceptance remains with the verifier.

`check_alert_card.py` covers 16 combinations (two languages × two appearances ×
four kinds), checking actual alert prototypes and outer-corner ownership in the
pinned VM. These tests deliberately make no GPU/rendering claim.
