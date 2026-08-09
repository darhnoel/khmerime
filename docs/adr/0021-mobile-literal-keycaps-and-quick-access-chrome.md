# ADR-0021: Mobile literal keycaps and Quick Access chrome

**Status:** Accepted

Android and iOS on-screen digit, punctuation, and symbol keys commit exactly the character printed on the key, even when a Khmer Composition is active; in that case the visible Khmer Composition commits first and the literal character follows. Legacy Khmer keycap mappings remain in the shared engine for desktop and physical-keyboard compatibility, while mobile exposes the mapped Khmer digits and marks directly through the Quick Access Tray instead. This trades hidden NiDA-style mobile conversions for a predictable visible-key contract without changing desktop behavior.

Mobile Khmer romanization keeps two fixed chrome rows across QWERTY, `123`, and `#+=`: unboxed Khmer digits above scrollable Khmer mark chips while idle, replaced by the Strip and candidate surface during Composition. CharPick keeps one candidate-row slot and reuses the mark row while idle; English Mode collapses both rows. Row count therefore changes only on an explicit input-mode transition, never as composition content appears or disappears.

Quick Access taps insert the exact Unicode character at the selection with no Composition or automatic spacing and leave the tray available for repeated input. Tray items use normal key haptics and pressed feedback but no Key Preview Popup. Android uses display-only dotted circles where its renderer needs them while committing only the raw mark. On Apple platforms, every isolated Khmer nonspacing mark is passed raw because the Khmer shaper already supplies its placeholder circle; prefixing `◌` produces a double circle.
