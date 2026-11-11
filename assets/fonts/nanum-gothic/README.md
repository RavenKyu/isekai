# Nanum Gothic Regular

- Unmodified `NanumGothic-Regular.ttf`, 2,054,744 bytes.
- Font name version: `Version 3.020;PS 1;hotconv 1.0.57;makeotf.lib2.0.21895`.
- Copyright © 2010 NHN Corporation; designed by Sandoll Communications Inc.
- License: SIL Open Font License 1.1. The complete upstream notice and license are in [OFL.txt](OFL.txt).
- Distribution: [google/fonts](https://github.com/google/fonts/tree/1c96438dec81b2b5078557d91315e31fb29b9462/ofl/nanumgothic), revision `1c96438dec81b2b5078557d91315e31fb29b9462`.
- Original font project: [googlefonts/nanumgothic](https://github.com/googlefonts/nanumgothic/tree/1893597aed63d9fecf2e84a70804f60ae490162d), as recorded in the distribution's `METADATA.pb`.
- Downloaded 2026-11-11 from these immutable URLs:
  - [TTF](https://raw.githubusercontent.com/google/fonts/1c96438dec81b2b5078557d91315e31fb29b9462/ofl/nanumgothic/NanumGothic-Regular.ttf)
  - [OFL](https://raw.githubusercontent.com/google/fonts/1c96438dec81b2b5078557d91315e31fb29b9462/ofl/nanumgothic/OFL.txt)
- SHA256:
  - TTF: `76f45ef4a6bcff344c837c95a7dcc26e017e38b5846d5ae0cdcb5b86be2e2d31`
  - OFL: `eeacf16032901d0ed0456876ec77b8f0fda6b3fecec7d972f8543eb602e6c30f`

The font covers basic Latin U+0020–U+007E and all 11,172 Hangul syllables U+AC00–U+D7A3.
`isekai-editor` embeds it with `include_bytes!` after egui's default fonts in both
Proportional and Monospace fallback lists. It does not make Hangul monospaced.
The default fonts supply accented Latin such as `é` and other symbols.
No system font installation or network request is needed at runtime. Non-dev `isekai`
does not depend on the editor and does not embed this file.

Verify the shipped files from the checkout root:

```sh
sha256sum assets/fonts/nanum-gothic/NanumGothic-Regular.ttf assets/fonts/nanum-gothic/OFL.txt
cargo test --locked -p isekai-editor fonts::tests
scripts/check-no-egui.sh
```
