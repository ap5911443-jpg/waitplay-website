//! Read-only checks against the real Vibe Play Supabase project (HTTPS through rustls). They send no email and create no account.
//! Run with:  cargo test -p waitplay-auth --test live -- --ignored --nocapture

use waitplay_auth::gotrue::Client;
use waitplay_auth::{AuthConfig, AuthError};

fn client() -> Client {
    Client::new(AuthConfig::from_build().expect("configured"))
}

#[test]
#[ignore = "talks to the real Supabase project"]
fn live_the_project_answers_over_https_and_rejects_unknown_credentials() {
    let c = client();
    let settings = c.call("GET", "/auth/v1/settings", None, None).expect("auth settings over HTTPS");
    assert_eq!(settings["external"]["email"], true, "email sign-in must be enabled in the dashboard");
    println!("providers: google={} apple={} mailer_autoconfirm={}", settings["external"]["google"], settings["external"]["apple"], settings["mailer_autoconfirm"]);

    let err = c.sign_in_password("nobody-live-check@example.invalid", "definitely-wrong-1").unwrap_err();
    assert!(matches!(&err, AuthError::Rejected { code, .. } if code == "invalid_credentials"), "{err:?}");
    assert_eq!(err.user_message(), "Wrong email or password.");

    // a refresh token that does not exist is a *definitive* answer, not a network error
    assert_eq!(c.refresh("not-a-real-refresh-token").unwrap_err(), AuthError::SessionExpired);
}

#[test]
#[ignore = "talks to the real Supabase project"]
fn live_row_level_security_hides_everything_from_anonymous_callers() {
    let c = client();
    for table in ["profiles", "entitlements", "devices", "user_preferences"] {
        match c.call("GET", &format!("/rest/v1/{table}?select=*"), None, None) {
            Ok(rows) => assert_eq!(rows.as_array().map(Vec::len), Some(0), "anonymous caller can read {table}: {rows}"),
            Err(AuthError::Rejected { code, .. }) => println!("{table}: refused ({code}), fine (not created yet, or no privilege)"),
            Err(e) => panic!("{table}: {e:?}"),
        }
    }
}
