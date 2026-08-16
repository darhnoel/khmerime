# Khmer Compact Keyboard Experiment

This context explores a compact touchscreen keyboard with a 3×5 core and an
**Auxiliary Family Row** for direct Khmer character entry. Its language is
experimental and does not define production KhmerIME behavior.

## Language

**Key Family**:
A center member and up to four direction members grouped on one character key.
_Avoid_: letter group, button group

**Auxiliary Family Row**:
A direct-entry surface of **Key Families** for the characters and shortcuts that
sit outside the corpus-optimized core grid.
_Avoid_: Quick Access Tray, scrolling tray, extra keyboard

**Directional Lean**:
A compact key gesture in which the thumb remains in contact and shifts subtly toward a displayed **Key Family** member; releasing commits that member, while returning to neutral restores the center member. A diagonal shift targets its dominant valid direction.
_Avoid_: flick, button leaning

**Lean Preview**:
A transient popup above the current touch point that appears immediately and
shows the complete **Key Family**, with the member currently targeted by a
**Directional Lean** highlighted. It combines family discovery with selection
feedback, especially when narrow-screen presentation hides or softens members.
_Avoid_: flick popup, candidate popup, five-way popup

**Neutral Zone**:
The small area around initial thumb contact in which a **Directional Lean** continues to target the centre member. Returning to this area restores the centre selection.
_Avoid_: dead zone

**Inward Gravity**:
The ordering rule within a fixed **Key Family**: its most frequent member takes
the centre, then more frequent remaining members receive directions that point
more strongly toward the keyboard centre. It never determines family membership.
_Avoid_: gravity rule, frequency grouping

**Entry Unit**:
The exact text produced by one Key Family selection. It may contain one or
multiple Unicode code points and is undone as one unit while still in preedit.
_Avoid_: character, code point, atomic shortcut
