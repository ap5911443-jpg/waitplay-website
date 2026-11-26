//! The Vibe Play logo (the pixel-art window with `</>` and a play triangle) as a pre-rendered 64x64 RGBA bitmap, resampled to
//! whatever size an icon needs. Straight (non-premultiplied) alpha, row-major, top row first.

const SRC: &[u8] = include_bytes!("../assets/logo64.rgba");
const SRC_N: usize = 64;

/// The logo as a `size` px square RGBA buffer. Area-averaged in premultiplied space, so shrinking (tray 32 px, menu bar 36 px)
/// keeps clean edges; sizes above 64 are stretched by the same weights.
pub fn rgba(size: u32) -> Vec<u8> {
    let n = size as usize;
    assert_eq!(SRC.len(), SRC_N * SRC_N * 4);
    if n == SRC_N {
        return SRC.to_vec();
    }
    let step = SRC_N as f32 / n as f32;
    let mut out = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let (x0, x1, y0, y1) = (x as f32 * step, (x + 1) as f32 * step, y as f32 * step, (y + 1) as f32 * step);
            let (mut r, mut g, mut b, mut a, mut wsum) = (0f32, 0f32, 0f32, 0f32, 0f32);
            for sy in (y0.floor() as usize)..(y1.ceil() as usize).min(SRC_N) {
                let wy = (y1.min(sy as f32 + 1.0) - y0.max(sy as f32)).max(0.0);
                for sx in (x0.floor() as usize)..(x1.ceil() as usize).min(SRC_N) {
                    let w = wy * (x1.min(sx as f32 + 1.0) - x0.max(sx as f32)).max(0.0);
                    let i = (sy * SRC_N + sx) * 4;
                    let al = SRC[i + 3] as f32 / 255.0;
                    r += SRC[i] as f32 * al * w;
                    g += SRC[i + 1] as f32 * al * w;
                    b += SRC[i + 2] as f32 * al * w;
                    a += al * w;
                    wsum += w;
                }
            }
            let o = (y * n + x) * 4;
            if a > 0.0 {
                out[o] = (r / a).round() as u8;
                out[o + 1] = (g / a).round() as u8;
                out[o + 2] = (b / a).round() as u8;
            }
            out[o + 3] = (a / wsum * 255.0).round() as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_resamples_to_any_size_with_transparent_corners_and_an_opaque_body() {
        for n in [16u32, 32, 36, 64, 128] {
            let px = rgba(n);
            assert_eq!(px.len(), (n * n * 4) as usize);
            assert_eq!(px[3], 0, "{n}px: the corner outside the rounded square is transparent");
            // a spot in the dark body, left of the centre of the emblem, is opaque
            let i = (((n / 2) * n + n / 8) * 4 + 3) as usize;
            assert_eq!(px[i], 255, "{n}px: body pixel is opaque");
        }
    }
}
