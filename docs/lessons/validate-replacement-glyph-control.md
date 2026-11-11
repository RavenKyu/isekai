# Validate the replacement glyph control before comparing labels

## What happened

- A font-removal mutation correctly failed the pixel check, but the scratch assertion expected the diagnostic for `é` and rejected the earlier missing-control diagnostic.

## Why

- A missing character uses the primary font's `.notdef` glyph. Nanum Gothic's replacement control renders blank when it becomes the only font; egui's usual primary fonts render a visible box.
- A missing glyph is not guaranteed to produce a nonempty tofu box. Counting light pixels or assuming that every font shares the same box is insufficient.

## Do this instead

- Render the unsupported control through the same family, context and GPU path as the samples.
- Require a usable control before comparing masks, reject blank samples, and prove each font-removal mutation fails the glyph check. Accept the check's actual failure diagnostic rather than assuming which missing glyph it reaches first.

Learned in: isk-kznl.2rx
