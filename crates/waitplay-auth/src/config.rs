//! Which Supabase project to talk to. Both values are public by design (the project URL and the publishable/anon key ship inside
//! every client; Row Level Security protects the data). The service-role key must never appear anywhere in this repository.

/// Build-time overrides: `WAITPLAY_SUPABASE_URL`, `WAITPLAY_SUPABASE_ANON_KEY`. The defaults are the Vibe Play project.
const URL: &str = match option_env!("WAITPLAY_SUPABASE_URL") {
    Some(u) => u,
    None => "https://gvuqncuejklrsaaphsrq.supabase.co",
};
const ANON_KEY: &str = match option_env!("WAITPLAY_SUPABASE_ANON_KEY") {
    Some(k) => k,
    None => "sb_publishable_Wd_j9K4fnsnXu0kxXrL8hQ_fR33gwWt",
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthConfig {
    pub url: String,
    pub anon_key: String,
}

impl AuthConfig {
    /// The configuration compiled into this build; `None` when it is empty or not an https Supabase URL.
    pub fn from_build() -> Option<AuthConfig> {
        let c = AuthConfig { url: URL.trim_end_matches('/').to_string(), anon_key: ANON_KEY.to_string() };
        c.is_valid().then_some(c)
    }

    /// `http://127.0.0.1:<port>` is accepted for tests against a local mock server only.
    pub fn is_valid(&self) -> bool {
        let local_test = self.url.starts_with("http://127.0.0.1:");
        (self.url.starts_with("https://") || local_test) && !self.anon_key.is_empty() && !self.anon_key.starts_with("sb_secret_") && !self.anon_key.contains("service_role")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_build_configuration_is_valid_and_public() {
        let c = AuthConfig::from_build().expect("configured");
        assert!(c.url.starts_with("https://") && c.url.ends_with(".supabase.co"), "{}", c.url);
        assert!(c.anon_key.starts_with("sb_publishable_") || c.anon_key.starts_with("eyJ"), "the app must only ever hold a publishable/anon key");
    }

    #[test]
    fn a_secret_key_is_refused() {
        for key in ["sb_secret_abc", "eyJ...service_role..."] {
            assert!(!AuthConfig { url: "https://x.supabase.co".into(), anon_key: key.into() }.is_valid(), "{key}");
        }
        assert!(!AuthConfig { url: "http://evil.example".into(), anon_key: "sb_publishable_x".into() }.is_valid());
    }
}
