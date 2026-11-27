//! Signing in with Google.
//!
//! The frontend completes Google's own sign-in and sends us the ID token;
//! Google itself checks the token's signature and expiry. Tests point
//! `google.tokeninfo_url` at a local server instead.

mod identity;

pub use identity::{verify_id_token, GoogleError, GoogleIdentity};
