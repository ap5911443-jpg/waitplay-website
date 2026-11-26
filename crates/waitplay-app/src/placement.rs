#![cfg_attr(not(target_os = "macos"), allow(dead_code))]
//! Where the floating widget and the entertainment window sit on a screen. Pure geometry in logical points with a TOP-LEFT origin
//! (y grows downwards), so every rule here is unit tested without a display.

use waitplay_core::settings::WidgetPosition;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct LRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl LRect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> LRect {
        LRect { x, y, w, h }
    }
    pub fn right(&self) -> f64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px < self.right() && py >= self.y && py < self.bottom()
    }
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

/// One display: its full frame, its usable area (menu bar and Dock excluded) and its backing scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub frame: LRect,
    pub work: LRect,
    pub scale: f64,
}


/// Top-left corner of a `size` window in the chosen corner (or, for the notch, the middle of the top edge) of `work`.
pub fn corner_origin(work: LRect, size: (f64, f64), pos: WidgetPosition, margin: f64) -> (f64, f64) {
    let left = work.x + margin;
    let right = work.right() - margin - size.0;
    let top = work.y + margin;
    let bottom = work.bottom() - margin - size.1;
    // never leave the usable area, even when the window is larger than it
    let centre = work.x + (work.w - size.0) / 2.0;
    let clamp = |v: f64, lo: f64, hi: f64| if hi < lo { lo } else { v.clamp(lo, hi) };
    let (x, y) = match pos {
        WidgetPosition::TopRight => (right, top),
        WidgetPosition::TopCenter => (centre, top),
        WidgetPosition::BottomRight => (right, bottom),
        WidgetPosition::BottomLeft => (left, bottom),
    };
    (clamp(x, work.x, work.right() - size.0), clamp(y, work.y, work.bottom() - size.1))
}

/// The entertainment window: centred on the display under the pointer, 70% x 82% of the usable area (at least 480 x 420, at most
/// 1400 x 950, never larger than the area), the same on macOS and Windows. The widget floats above it.
pub fn centered_companion_rect(work: LRect) -> LRect {
    let w = (work.w * 0.70).clamp(480.0, 1400.0).min(work.w);
    let h = (work.h * 0.82).clamp(420.0, 950.0).min(work.h);
    LRect::new(work.x + (work.w - w) / 2.0, work.y + (work.h - h) / 2.0, w, h)
}

/// The screen a point is on; the first screen (the primary display) when it is on none (a display was unplugged).
pub fn screen_at(screens: &[Screen], px: f64, py: f64) -> Option<usize> {
    screens.iter().position(|s| s.frame.contains(px, py)).or(if screens.is_empty() { None } else { Some(0) })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: LRect = LRect { x: 0.0, y: 25.0, w: 1440.0, h: 800.0 };

    #[test]
    fn widget_sits_in_each_of_the_three_corners() {
        let size = (200.0, 56.0);
        assert_eq!(corner_origin(WORK, size, WidgetPosition::TopRight, 8.0), (1232.0, 33.0));
        assert_eq!(corner_origin(WORK, size, WidgetPosition::BottomRight, 8.0), (1232.0, 761.0));
        assert_eq!(corner_origin(WORK, size, WidgetPosition::BottomLeft, 8.0), (8.0, 761.0));
    }

    #[test]
    fn the_notch_hangs_at_the_middle_of_the_top_edge_and_stays_centred_as_it_grows() {
        for w in [148.0, 392.0] {
            let (x, y) = corner_origin(WORK, (w, 40.0), WidgetPosition::TopCenter, 0.0);
            assert_eq!((x, y), ((1440.0 - w) / 2.0, 25.0), "width {w}");
        }
        // on a display left of the primary the centre moves with it
        let work = LRect::new(-1920.0, 0.0, 1920.0, 1080.0);
        assert_eq!(corner_origin(work, (400.0, 40.0), WidgetPosition::TopCenter, 0.0), (-1160.0, 0.0));
    }

    #[test]
    fn secondary_display_with_an_offset_origin() {
        // a display left of the primary: negative x; one above: negative y
        let work = LRect::new(-1920.0, -200.0, 1920.0, 1080.0);
        assert_eq!(corner_origin(work, (200.0, 56.0), WidgetPosition::BottomLeft, 8.0), (-1912.0, 816.0));
        assert_eq!(corner_origin(work, (200.0, 56.0), WidgetPosition::TopRight, 8.0), (-208.0, -192.0));
    }

    #[test]
    fn a_window_bigger_than_the_screen_stays_pinned_to_the_origin() {
        let tiny = LRect::new(100.0, 100.0, 150.0, 40.0);
        assert_eq!(corner_origin(tiny, (200.0, 56.0), WidgetPosition::BottomRight, 8.0), (100.0, 100.0));
    }

    #[test]
    fn companion_is_centred_70_by_82_percent_and_never_leaves_the_screen() {
        let r = centered_companion_rect(WORK);
        let near = |a: f64, b: f64| (a - b).abs() < 1e-6;
        assert!(near(r.w, 1008.0) && near(r.h, 656.0), "{:?}", r);
        assert!(near(r.x, 216.0) && near(r.y, 25.0 + 72.0), "{:?}", r);
        let small = LRect::new(0.0, 0.0, 300.0, 400.0);
        let r = centered_companion_rect(small);
        assert!(r.x >= 0.0 && r.y >= 0.0 && r.right() <= 300.0 && r.bottom() <= 400.0, "{:?}", r);
        let huge = centered_companion_rect(LRect::new(0.0, 0.0, 5120.0, 2880.0));
        assert_eq!((huge.w, huge.h), (1400.0, 950.0), "capped like on Windows");
    }

    #[test]
    fn screen_lookup_falls_back_to_the_primary() {
        let a = Screen { frame: LRect::new(0.0, 0.0, 1000.0, 800.0), work: LRect::new(0.0, 25.0, 1000.0, 775.0), scale: 2.0 };
        let b = Screen { frame: LRect::new(1000.0, 0.0, 1000.0, 800.0), work: LRect::new(1000.0, 0.0, 1000.0, 800.0), scale: 1.0 };
        assert_eq!(screen_at(&[a, b], 1500.0, 10.0), Some(1));
        assert_eq!(screen_at(&[a, b], 5000.0, 10.0), Some(0), "unplugged display");
        assert_eq!(screen_at(&[], 0.0, 0.0), None);
    }
}
