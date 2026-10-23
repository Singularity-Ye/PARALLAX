//! Common classification for authenticated Steam GET responses.
use crate::{application::ports::SteamReadError as E, domain::accounts::Credentials};
use reqwest::{StatusCode, header::HeaderMap};
pub(super) fn classify(
    status: StatusCode,
    headers: &HeaderMap,
    secret: &Credentials,
) -> Result<(), E> {
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(E::RateLimited(
            headers
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(300)
                .clamp(300, 604800),
        ));
    }
    if status == StatusCode::UNAUTHORIZED {
        return Err(E::Expired);
    }
    if status.is_redirection() {
        // Do not follow even same-host redirects, and never forward Cookie to their destination.
        let login = headers
            .get("location")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| {
                reqwest::Url::parse("https://steamcommunity.com/")
                    .ok()?
                    .join(s)
                    .ok()
            })
            .is_some_and(|u| {
                u.scheme() == "https"
                    && u.host_str() == Some("steamcommunity.com")
                    && u.path().starts_with("/login")
            });
        return Err(if login { E::Expired } else { E::Rejected });
    }
    if status != StatusCode::OK {
        return Err(E::Rejected);
    }
    for cookie in headers.get_all("set-cookie") {
        let pair = cookie
            .to_str()
            .map_err(|_| E::InvalidPayload)?
            .split(';')
            .next()
            .unwrap_or_default();
        if let Some((name, value)) = pair.split_once('=')
            && ["steamLoginSecure", "sessionid"].contains(&name.trim())
            && secret.cookie(name.trim()) != Some(value)
        {
            return Err(if name.trim() == "steamLoginSecure" && value.is_empty() {
                E::Expired
            } else {
                E::CredentialsChanged
            });
        }
    }
    Ok(())
}
