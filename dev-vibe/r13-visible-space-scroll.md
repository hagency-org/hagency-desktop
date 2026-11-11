# R13: discoverable Space overflow

The Space PortalList now keeps a six-logical-pixel thumb visible at rest on the navy rail, with a 28-pixel minimum handle and stronger hover/drag contrast. It uses the existing PortalList for both the desktop vertical and mobile horizontal orientations, retaining wheel/trackpad/drag and focused Home/End/Page navigation. Home/Add controls remain outside the list.

A headless native drawing test instantiates the actual adaptive SpacesBar with 0, 1 and 20 spaces in both orientations and effective logical viewports corresponding to 100/125/150% zoom. It confirms overflow, a drawn handle, and real focused End/Home events reaching the last item and returning to the first. The test passed across all 18 cases. This is native layout/input verification, not an OS screenshot or a substitute for mouse/trackpad qualification on every platform. Initial fixture failures (missing script prelude; key focus not committed by platform dispatch) are retained in .run and corrected in the harness.

Duplication checked: #50/#52 cover Space filters/avatars, #53 redraw/fonts, #65 dependency pin, #67 required marker and #69 discard-alert card. None fixes the hidden Space overflow handle. Stacked on R11 #70 to keep prior session fixes. The release runner includes the overflow test as a required non-empty gate.
