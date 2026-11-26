//! The sites Vibe Play hosts, their home pages and their real logos.
//!
//! Logo geometry: Simple Icons (CC0), 24x24 viewBox. Colours are each brand's published colour. One place to add a site:
//! an entry in [`SITES`] and a branch in [`logo_svg`].

use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, PartialEq)]
pub struct Site {
    /// Value of `--site=`, the single-instance name and the hand-off file name.
    pub key: &'static str,
    pub label: &'static str,
    /// Home page, opened first and by a click on the site's own logo.
    pub url: &'static str,
    /// A page on one of these hosts (or a subdomain of it) belongs to the site.
    pub hosts: &'static [&'static str],
    pub aliases: &'static [&'static str],
}

pub const SITES: [Site; 3] = [
    Site { key: "instagram", label: "Instagram", url: "https://www.instagram.com/", hosts: &["instagram.com"], aliases: &["ig"] },
    Site { key: "youtube", label: "YouTube", url: "https://www.youtube.com/", hosts: &["youtube.com", "youtu.be"], aliases: &["yt"] },
    Site { key: "x", label: "X", url: "https://x.com/home", hosts: &["x.com", "twitter.com"], aliases: &["twitter"] },
];

/// The site agent sessions belong to: its companion also owns the launch and close hand-off files.
pub const DEFAULT_KEY: &str = "instagram";

pub fn default_site() -> &'static Site {
    &SITES[0]
}

/// The platform the user prefers (Settings > Entertainment); Instagram until they choose.
pub fn preferred_site() -> &'static Site {
    by_name(&waitplay_core::settings::Settings::load().preferred_platform).unwrap_or_else(default_site)
}

/// By `--site=` value (key or alias, any case).
pub fn by_name(name: &str) -> Option<&'static Site> {
    let n = name.trim().to_lowercase();
    SITES.iter().find(|s| s.key == n || s.aliases.contains(&n.as_str()))
}

/// The site a page belongs to; `None` for any other address.
pub fn for_url(url: &str) -> Option<&'static Site> {
    let rest = url.split_once("://").map(|(_, r)| r)?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit('@').next()?;
    let host = authority.split(':').next()?.to_lowercase();
    SITES.iter().find(|s| s.hosts.iter().any(|h| host == *h || host.ends_with(&format!(".{}", h))))
}

const INSTAGRAM: &str = "M7.0301.084c-1.2768.0602-2.1487.264-2.911.5634-.7888.3075-1.4575.72-2.1228 1.3877-.6652.6677-1.075 1.3368-1.3802 2.127-.2954.7638-.4956 1.6365-.552 2.914-.0564 1.2775-.0689 1.6882-.0626 4.947.0062 3.2586.0206 3.6671.0825 4.9473.061 1.2765.264 2.1482.5635 2.9107.308.7889.72 1.4573 1.388 2.1228.6679.6655 1.3365 1.0743 2.1285 1.38.7632.295 1.6361.4961 2.9134.552 1.2773.056 1.6884.069 4.9462.0627 3.2578-.0062 3.668-.0207 4.9478-.0814 1.28-.0607 2.147-.2652 2.9098-.5633.7889-.3086 1.4578-.72 2.1228-1.3881.665-.6682 1.0745-1.3378 1.3795-2.1284.2957-.7632.4966-1.636.552-2.9124.056-1.2809.0692-1.6898.063-4.948-.0063-3.2583-.021-3.6668-.0817-4.9465-.0607-1.2797-.264-2.1487-.5633-2.9117-.3084-.7889-.72-1.4568-1.3876-2.1228C21.2982 1.33 20.628.9208 19.8378.6165 19.074.321 18.2017.1197 16.9244.0645 15.6471.0093 15.236-.005 11.977.0014 8.718.0076 8.31.0215 7.0301.0839m.1402 21.6932c-1.17-.0509-1.8053-.2453-2.2287-.408-.5606-.216-.96-.4771-1.3819-.895-.422-.4178-.6811-.8186-.9-1.378-.1644-.4234-.3624-1.058-.4171-2.228-.0595-1.2645-.072-1.6442-.079-4.848-.007-3.2037.0053-3.583.0607-4.848.05-1.169.2456-1.805.408-2.2282.216-.5613.4762-.96.895-1.3816.4188-.4217.8184-.6814 1.3783-.9003.423-.1651 1.0575-.3614 2.227-.4171 1.2655-.06 1.6447-.072 4.848-.079 3.2033-.007 3.5835.005 4.8495.0608 1.169.0508 1.8053.2445 2.228.408.5608.216.96.4754 1.3816.895.4217.4194.6816.8176.9005 1.3787.1653.4217.3617 1.056.4169 2.2263.0602 1.2655.0739 1.645.0796 4.848.0058 3.203-.0055 3.5834-.061 4.848-.051 1.17-.245 1.8055-.408 2.2294-.216.5604-.4763.96-.8954 1.3814-.419.4215-.8181.6811-1.3783.9-.4224.1649-1.0577.3617-2.2262.4174-1.2656.0595-1.6448.072-4.8493.079-3.2045.007-3.5825-.006-4.848-.0608M16.953 5.5864A1.44 1.44 0 1 0 18.39 4.144a1.44 1.44 0 0 0-1.437 1.4424M5.8385 12.012c.0067 3.4032 2.7706 6.1557 6.173 6.1493 3.4026-.0065 6.157-2.7701 6.1506-6.1733-.0065-3.4032-2.771-6.1565-6.174-6.1498-3.403.0067-6.156 2.771-6.1496 6.1738M8 12.0077a4 4 0 1 1 4.008 3.9921A3.9996 3.9996 0 0 1 8 12.0077";
const YOUTUBE: &str = "M23.498 6.186a3.016 3.016 0 0 0-2.122-2.136C19.505 3.545 12 3.545 12 3.545s-7.505 0-9.377.505A3.017 3.017 0 0 0 .502 6.186C0 8.07 0 12 0 12s0 3.93.502 5.814a3.016 3.016 0 0 0 2.122 2.136c1.871.505 9.376.505 9.376.505s7.505 0 9.377-.505a3.015 3.015 0 0 0 2.122-2.136C24 15.93 24 12 24 12s0-3.93-.502-5.814zM9.545 15.568V8.432L15.818 12l-6.273 3.568z";
const YOUTUBE_PLAY: &str = "M9.545 15.568V8.432L15.818 12l-6.273 3.568z";
const X: &str = "M18.901 1.153h3.68l-8.04 9.19L24 22.846h-7.406l-5.8-7.584-6.638 7.584H.474l8.6-9.83L0 1.154h7.594l5.243 6.932ZM17.61 20.644h2.039L6.486 3.24H4.298Z";

static GRADIENT_ID: AtomicU32 = AtomicU32::new(0);

/// Inline `<svg>` of the real logo, `size` px square. `on_dark`: X is white on a dark surface and black on a light one.
pub fn logo_svg(key: &str, size: u32, on_dark: bool) -> String {
    match key {
        "instagram" => {
            // Each inline copy needs its own gradient id: two logos sharing one id break when the first is hidden
            let id = format!("wp-ig-{}", GRADIENT_ID.fetch_add(1, Ordering::Relaxed));
            format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='{s}' height='{s}' viewBox='0 0 24 24' aria-hidden='true'><defs><linearGradient id='{id}' x1='0.1' y1='1' x2='0.9' y2='0'><stop offset='0' stop-color='#FEDA75'/><stop offset='.25' stop-color='#FA7E1E'/><stop offset='.5' stop-color='#D62976'/><stop offset='.75' stop-color='#962FBF'/><stop offset='1' stop-color='#4F5BD5'/></linearGradient></defs><path fill='url(#{id})' fill-rule='nonzero' d='{d}'/></svg>",
                s = size, id = id, d = INSTAGRAM
            )
        }
        "youtube" => format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{s}' height='{s}' viewBox='0 0 24 24' aria-hidden='true'><path fill='#FFFFFF' d='{p}'/><path fill='#FF0000' fill-rule='nonzero' d='{d}'/></svg>",
            s = size, p = YOUTUBE_PLAY, d = YOUTUBE
        ),
        "x" => format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{s}' height='{s}' viewBox='0 0 24 24' aria-hidden='true'><path fill='{c}' d='{d}'/></svg>",
            s = size, c = if on_dark { "#FFFFFF" } else { "#000000" }, d = X
        ),
        _ => String::new(),
    }
}

/// Like [`logo_svg`], but X follows the surrounding text colour, so it reads on a light header and on a dark one.
pub fn logo_svg_adaptive(key: &str, size: u32) -> String {
    let svg = logo_svg(key, size, true);
    if key == "x" {
        svg.replace("fill='#FFFFFF'", "fill='currentColor'")
    } else {
        svg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup() {
        assert_eq!(by_name("YT").unwrap().key, "youtube");
        assert_eq!(by_name(" twitter ").unwrap().key, "x");
        assert!(by_name("reddit").is_none());
        assert_eq!(for_url("https://m.youtube.com/watch?v=1").unwrap().key, "youtube");
        assert_eq!(for_url("https://www.instagram.com/").unwrap().key, "instagram");
        assert_eq!(for_url("https://notx.com/").is_none(), true);
        assert_eq!(for_url("https://x.com:443/home").unwrap().key, "x");
        assert!(for_url("https://accounts.google.com/").is_none());
        assert!(for_url("about:blank").is_none());
    }

    #[test]
    fn logos_exist_for_every_site() {
        for s in SITES.iter() {
            assert!(logo_svg(s.key, 18, true).contains("<path"), "{}", s.key);
        }
    }

    #[test]
    fn adaptive_logo_follows_the_text_colour_only_for_x() {
        assert!(logo_svg_adaptive("x", 18).contains("fill='currentColor'"));
        assert!(!logo_svg_adaptive("x", 18).contains("#FFFFFF"));
        // YouTube's play triangle must stay white on its red badge
        assert_eq!(logo_svg_adaptive("youtube", 18), logo_svg("youtube", 18, true));
    }
}
