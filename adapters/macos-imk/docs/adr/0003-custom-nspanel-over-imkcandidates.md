# Custom NSPanel for candidate display instead of IMKCandidates

The candidate window is a custom non-activating `NSPanel` owned by the Swift host shell,
not the system `IMKCandidates` class. `IMKCandidates` only supports a flat candidate
list; it cannot render segment chips (the phrase bar showing each segment's Khmer output
with a ✏ edit button). Because segment chips are a first-class UX requirement shared with
the IBus and iOS adapters, `IMKCandidates` cannot meet the spec.

The custom panel renders the same two-row layout used by the iOS `CandidatePanelView`
(chips row + candidates row) using AppKit `NSStackView`. It is positioned below the
cursor using `firstRectForCharacterRange:actualRange:` from the `IMKTextInput` client
protocol, and uses `NSWindowStyleMask.nonActivatingPanel` so it never steals focus from
the host application.

The tradeoff is that the panel appearance does not inherit system IME styling and requires
manual layout. This is accepted because the structured segment chip UX is a deliberate
product decision and cannot be retrofitted into `IMKCandidates`.
