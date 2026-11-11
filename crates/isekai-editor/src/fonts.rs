//! Embedded editor fonts, shared by native, web and tests.

use egui::{Context, FontData, FontDefinitions, FontFamily};

const KOREAN_FONT: &[u8] =
    include_bytes!("../../../assets/fonts/nanum-gothic/NanumGothic-Regular.ttf");

pub fn context() -> Context {
    let ctx = Context::default();
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "NanumGothic".into(),
        FontData::from_static(KOREAN_FONT).into(),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("NanumGothic".into());
    }
    ctx.set_fonts(fonts);
    ctx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_families_have_real_latin_and_every_hangul_syllable() {
        let ctx = context();
        let mut output = ctx.run_pass(Default::default(), |ctx| {
            ctx.fonts_mut(|fonts| {
                for family in [FontFamily::Proportional, FontFamily::Monospace] {
                    let id = egui::FontId::new(32.0, family);
                    for c in ['A', 'é', '가', '힣', '√', '×'] {
                        assert!(fonts.has_glyph(&id, c), "missing {c:?} in {id:?}");
                    }
                    for code in 0xAC00..=0xD7A3 {
                        let c = char::from_u32(code).unwrap();
                        assert!(fonts.has_glyph(&id, c), "missing {c:?} in {id:?}");
                    }
                    assert!(!fonts.has_glyph(&id, '\u{10FFFF}'));
                }
            });
        });
        output.textures_delta.clear();
    }

    #[test]
    fn korean_asset_has_full_hangul_and_basic_latin_but_cannot_mask_missing_defaults() {
        let mut fonts = FontDefinitions::empty();
        fonts.font_data.insert(
            "Korean only".into(),
            FontData::from_static(KOREAN_FONT).into(),
        );
        fonts
            .families
            .insert(FontFamily::Proportional, vec!["Korean only".into()]);
        let ctx = Context::default();
        ctx.set_fonts(fonts);
        let mut output = ctx.run_pass(Default::default(), |ctx| {
            ctx.fonts_mut(|fonts| {
                let id = egui::FontId::proportional(32.0);
                for code in (0x20..=0x7E).chain(0xAC00..=0xD7A3) {
                    assert!(fonts.has_glyph(&id, char::from_u32(code).unwrap()));
                }
                // The smoke's é must depend on egui's built-in fonts.
                assert!(!fonts.has_glyph(&id, 'é'));
            });
        });
        output.textures_delta.clear();
    }

    #[test]
    fn pasted_korean_reaches_text_edit() {
        let ctx = context();
        let mut text = String::new();
        let id = egui::Id::unique("Korean paste");
        for events in [vec![], vec![egui::Event::Paste("한글 가힣".into())]] {
            let input = egui::RawInput {
                events,
                ..Default::default()
            };
            let mut output = ctx.run_pass(input, |ctx| {
                egui::Window::new("한글 입력").show(ctx, |ui| {
                    ui.add(egui::TextEdit::singleline(&mut text).id(id))
                        .request_focus();
                });
            });
            output.textures_delta.clear();
        }
        assert_eq!(text, "한글 가힣");
    }
}
