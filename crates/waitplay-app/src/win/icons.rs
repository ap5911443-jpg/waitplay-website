//! The app icon: the Vibe Play logo, resampled to the size asked for. Pure RGBA buffers, used for the tray icon and the settings
//! window icon.

/// `size` px square straight-alpha RGBA.
pub fn app_icon_rgba(size: u32) -> Vec<u8> {
    crate::logo::rgba(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_has_the_right_size_and_a_transparent_corner() {
        let px = app_icon_rgba(32);
        assert_eq!(px.len(), 32 * 32 * 4);
        assert_eq!(px[3], 0, "the corner outside the rounded square is transparent");
    }
}
