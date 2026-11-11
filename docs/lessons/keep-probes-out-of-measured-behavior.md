# Keep diagnostic probes out of the behavior they measure

## What happened

- Review finding 1 in isk-kznl.p7z showed that removing the real shell draw still passed the dev smoke: the glyph strip supplied both UI vertices and lit pixels.

## Why

- The diagnostic strip shared the shell's counters. Keeping the old assertions in code did not preserve their ability to detect a missing shell.

## Do this instead

- Count and judge the application behavior separately from diagnostic geometry and pixels.
- Exclude probe geometry from shell vertex counts and clear the probe region in the readback copy after validating it.
- Remove the real application draw as a mutation check; the original smoke conditions must still fail.

Learned in: isk-kznl.2rx
