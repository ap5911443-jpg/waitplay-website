//! Every failure the auth layer can report, with a message that is safe and useful to show to the user.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// No internet, DNS failure, timeout, TLS failure. Never signs anybody out.
    Network(String),
    /// The server is up but failing (5xx). Same treatment as `Network`.
    Unavailable,
    /// Too many requests (429) or an email rate limit.
    RateLimited,
    /// The refresh token is no longer valid (expired, revoked, reused): the user must sign in again.
    SessionExpired,
    /// A 4xx answer from Supabase with its own message (wrong password, bad code, ...).
    Rejected { code: String, message: String },
    /// The browser flow was cancelled, timed out or came back with an invalid callback.
    Callback(String),
    /// The operating system secret store refused or failed.
    Storage(String),
    /// Authentication is not configured in this build.
    NotConfigured,
    /// The caller used the API in the wrong order (for example sign-out while signed out).
    State(&'static str),
}

impl AuthError {
    /// True for failures that say nothing about the session itself (offline, server down).
    pub fn is_transient(&self) -> bool {
        matches!(self, AuthError::Network(_) | AuthError::Unavailable | AuthError::RateLimited)
    }

    /// Text for the sign-in window. Never contains a token, a URL with a code or a stack of internals.
    pub fn user_message(&self) -> String {
        match self {
            AuthError::Network(_) => "No connection. Check your internet and try again. Vibe Play keeps working offline.".into(),
            AuthError::Unavailable => "The sign-in service is not responding. Try again in a moment.".into(),
            AuthError::RateLimited => "Too many attempts. Wait a minute and try again.".into(),
            AuthError::SessionExpired => "Your session expired. Please sign in again.".into(),
            AuthError::Rejected { code, message } => match code.as_str() {
                "invalid_credentials" => "Wrong email or password.".into(),
                "email_not_confirmed" => "Confirm your email first: enter the code we sent you.".into(),
                "user_already_exists" | "email_exists" => "An account with this email already exists. Sign in instead.".into(),
                "otp_expired" => "That code expired or is not valid. Request a new one.".into(),
                "weak_password" => "Choose a stronger password (at least 8 characters).".into(),
                "over_email_send_rate_limit" => "Too many emails were requested. While Vibe Play is in testing, Supabase's built-in mail service allows only about 2 per hour. Try again later, or sign in another way.".into(),
                "over_request_rate_limit" => "Too many attempts. Wait a minute and try again.".into(),
                "email_address_not_authorized" => "This address can't receive emails yet: while Vibe Play is in testing, emails are sent only to the project team's addresses. Use Google, or a password account, instead.".into(),
                "email_address_invalid" => "That email address isn't accepted. Check it for typos.".into(),
                "otp_disabled" | "user_not_found" => "No account exists for that email yet. Choose Create account first.".into(),
                _ if message.to_lowercase().contains("not authorized") => "This address can't receive emails yet: while Vibe Play is in testing, emails are sent only to the project team's addresses. Use Google, or a password account, instead.".into(),
                "signup_disabled" => "New accounts are disabled right now.".into(),
                "same_password" => "The new password must be different from the old one.".into(),
                _ if !message.is_empty() => message.clone(),
                _ => "The request was refused.".into(),
            },
            AuthError::Callback(m) => m.clone(),
            AuthError::Storage(_) => "Vibe Play could not use the system keychain to store your sign-in.".into(),
            AuthError::NotConfigured => "Sign-in is not available in this build.".into(),
            AuthError::State(m) => (*m).into(),
        }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl std::error::Error for AuthError {}

pub type Result<T> = std::result::Result<T, AuthError>;
