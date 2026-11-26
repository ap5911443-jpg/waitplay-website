//! Menu bar icon: the Vibe Play logo as a template image.

/// The logo as a black-on-transparent template image (macOS tints it for light and dark menu bars): the dark body of the emblem
/// becomes ink and its light window, `</>` and play glyphs become cut-outs. `size` px square (use 36 for an 18pt @2x status item).
pub fn menu_bar_rgba(size: u32) -> Vec<u8> {
    let mut px = crate::logo::rgba(size);
    for p in px.chunks_exact_mut(4) {
        let lum = (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) / 255.0;
        // dark (~0.23) stays solid, light (~0.9) disappears, with a soft ramp between
        let ink = (1.0 - ((lum - 0.35) / 0.3).clamp(0.0, 1.0)) * (p[3] as f32 / 255.0);
        p[0] = 0;
        p[1] = 0;
        p[2] = 0;
        p[3] = (ink * 255.0).round() as u8;
    }
    px
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_bar_icon_is_a_black_template_with_the_glyphs_cut_out() {
        let px = menu_bar_rgba(36);
        assert_eq!(px.len(), 36 * 36 * 4);
        assert!(px.chunks(4).all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0), "template images are pure black");
        let ink = px.chunks(4).filter(|p| p[3] > 128).count();
        assert!(ink > 400 && ink < 1100, "ink pixels: {ink}");
        assert_eq!(px[3], 0, "corner stays transparent");
    }
}
