# Khmer Compact Keyboard Experiment

This context explores a compact 3×5 touchscreen keyboard for direct Khmer character entry. Its language is experimental and does not define production KhmerIME behavior.

## Language

**Key Family**:
A center member and up to four direction members grouped on one character key.
_Avoid_: letter group, button group

**Directional Lean**:
A compact key gesture in which the thumb remains in contact and shifts subtly toward a displayed **Key Family** member; releasing commits that member, while returning to neutral restores the center member. A diagonal shift targets its dominant valid direction.
_Avoid_: flick, button leaning

**Lean Preview**:
A transient popup above the current touch point that appears immediately and shows the single **Key Family** member currently targeted by a **Directional Lean**. It is selection feedback, not a list of all available directions.
_Avoid_: flick popup, candidate popup, five-way popup

**Neutral Zone**:
The small area around initial thumb contact in which a **Directional Lean** continues to target the centre member. Returning to this area restores the centre selection.
_Avoid_: dead zone
