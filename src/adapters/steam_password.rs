use crate::domain::accounts::Credentials;
use anyhow::{Context, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;
use std::time::Duration;

fn encrypt(modulus: &str, exponent: &str, password: &str) -> anyhow::Result<String> {
    use windows_sys::Win32::Security::Cryptography::*;
    fn hex(s: &str) -> anyhow::Result<Vec<u8>> {
        ensure!(s.is_ascii() && s.len() <= 1024, "Invalid RSA parameter");
        let padded = if s.len() % 2 == 1 {
            format!("0{s}")
        } else {
            s.into()
        };
        (0..padded.len())
            .step_by(2)
            .map(|i| Ok(u8::from_str_radix(&padded[i..i + 2], 16)?))
            .collect()
    }
    let n = hex(modulus)?;
    let e = hex(exponent)?;
    ensure!(
        (256..=512).contains(&n.len()) && (1..=4).contains(&e.len()),
        "Invalid RSA key size"
    );
    let mut blob = Vec::new();
    for field in [
        BCRYPT_RSAPUBLIC_MAGIC,
        (n.len() * 8) as u32,
        e.len() as u32,
        n.len() as u32,
        0,
        0,
    ] {
        blob.extend(field.to_le_bytes());
    }
    blob.extend(e);
    blob.extend(&n);
    let mut algorithm = std::ptr::null_mut();
    let mut key = std::ptr::null_mut();
    let mut output = vec![0; n.len()];
    let mut written = 0;
    // CNG owns both handles. Release each after its last operation.
    unsafe {
        ensure!(
            BCryptOpenAlgorithmProvider(&mut algorithm, BCRYPT_RSA_ALGORITHM, std::ptr::null(), 0)
                == 0,
            "RSA provider unavailable"
        );
        let status = BCryptImportKeyPair(
            algorithm,
            std::ptr::null_mut(),
            BCRYPT_RSAPUBLIC_BLOB,
            &mut key,
            blob.as_ptr(),
            blob.len() as u32,
            0,
        );
        let encrypted = if status == 0 {
            let result = BCryptEncrypt(
                key,
                password.as_ptr(),
                password.len() as u32,
                std::ptr::null(),
                std::ptr::null_mut(),
                0,
                output.as_mut_ptr(),
                output.len() as u32,
                &mut written,
                BCRYPT_PAD_PKCS1,
            );
            BCryptDestroyKey(key);
            result == 0
        } else {
            false
        };
        BCryptCloseAlgorithmProvider(algorithm, 0);
        ensure!(encrypted, "Steam password encryption failed");
    }
    Ok(STANDARD.encode(&output[..written as usize]))
}

async fn post(
    client: &reqwest::Client,
    url: &str,
    fields: &[(&str, &str)],
) -> anyhow::Result<reqwest::Response> {
    let body = {
        let mut encoded = form_urlencoded::Serializer::new(String::new());
        encoded.extend_pairs(fields.iter().copied());
        encoded.finish()
    };
    Ok(client
        .post(url)
        .header("origin", "https://steamcommunity.com")
        .header("referer", "https://steamcommunity.com/")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?
        .error_for_status()?)
}
fn field<'a>(data: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    data[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .context("Steam response is missing a required field")
}

pub async fn login<F: std::future::Future<Output = Option<String>> + Send>(
    username: &str,
    password: &str,
    steam_id: &str,
    guard: impl FnOnce() -> F,
) -> anyhow::Result<Credentials> {
    const API: &str = "https://api.steampowered.com/IAuthenticationService";
    let client = super::network::client_builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()?;
    let rsa: Value = client
        .get(format!("{API}/GetPasswordRSAPublicKey/v1"))
        .query(&[("account_name", username)])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let rsa = &rsa["response"];
    let encrypted = encrypt(
        field(rsa, "publickey_mod")?,
        field(rsa, "publickey_exp")?,
        password,
    )?;
    let begin: Value = post(
        &client,
        &format!("{API}/BeginAuthSessionViaCredentials/v1"),
        &[
            ("account_name", username),
            ("encrypted_password", &encrypted),
            ("encryption_timestamp", field(rsa, "timestamp")?),
            ("persistence", "1"),
            ("website_id", "Community"),
            ("platform_type", "2"),
        ],
    )
    .await?
    .json()
    .await?;
    let begin = &begin["response"];
    ensure!(
        field(begin, "steamid")? == steam_id,
        "Steam identity mismatch"
    );
    let client_id = field(begin, "client_id")?;
    let request_id = field(begin, "request_id")?;
    let allowed = begin["allowed_confirmations"]
        .as_array()
        .context("Steam challenge missing")?;
    if allowed.iter().any(|v| v["confirmation_type"] == 3) {
        let code = guard().await.context("Steam Guard code unavailable")?;
        let response = post(
            &client,
            &format!("{API}/UpdateAuthSessionWithSteamGuardCode/v1"),
            &[
                ("client_id", client_id),
                ("steamid", steam_id),
                ("code_type", "3"),
                ("code", &code),
            ],
        )
        .await?;
        ensure!(
            response
                .headers()
                .get("x-eresult")
                .is_none_or(|value| value == "1"),
            "Steam Guard rejected"
        );
    } else {
        ensure!(
            allowed.iter().any(|v| v["confirmation_type"] == 1),
            "Steam requires interactive confirmation"
        );
    }
    let interval = begin["interval"].as_f64().unwrap_or(5.0);
    ensure!(
        (0.0..=30.0).contains(&interval),
        "Invalid Steam polling interval"
    );
    let mut token = None;
    for _ in 0..5 {
        tokio::time::sleep(Duration::from_secs_f64(interval.max(1.0))).await;
        let poll: Value = post(
            &client,
            &format!("{API}/PollAuthSessionStatus/v1"),
            &[("client_id", client_id), ("request_id", request_id)],
        )
        .await?
        .json()
        .await?;
        if let Some(value) = poll["response"]["refresh_token"]
            .as_str()
            .filter(|s| !s.is_empty())
        {
            token = Some(value.to_owned());
            break;
        }
    }
    let token = token.context("Steam login did not complete")?;
    let session = uuid::Uuid::new_v4().simple().to_string()[..24].to_owned();
    let finalized: Value = post(
        &client,
        "https://login.steampowered.com/jwt/finalizelogin",
        &[
            ("nonce", &token),
            ("sessionid", &session),
            ("redir", "https://steamcommunity.com/"),
        ],
    )
    .await?
    .json()
    .await?;
    ensure!(
        field(&finalized, "steamID")? == steam_id,
        "Steam transfer identity mismatch"
    );
    let transfers = finalized["transfer_info"]
        .as_array()
        .context("Steam cookie transfer unavailable")?;
    for transfer in transfers {
        let url = reqwest::Url::parse(field(transfer, "url")?)?;
        if url.scheme() != "https"
            || url.host_str() != Some("steamcommunity.com")
            || url.port().is_some()
            || url.path() != "/login/settoken"
            || !url.username().is_empty()
            || url.password().is_some()
        {
            continue;
        }
        let params = &transfer["params"];
        let response = post(
            &client,
            url.as_str(),
            &[
                ("steamID", steam_id),
                ("nonce", field(params, "nonce")?),
                ("auth", field(params, "auth")?),
            ],
        )
        .await?;
        for header in response.headers().get_all("set-cookie") {
            if let Some(cookie) = header
                .to_str()
                .ok()
                .and_then(|v| v.split(';').next())
                .filter(|v| v.starts_with("steamLoginSecure="))
            {
                return Ok(Credentials {
                    cookies: format!("{cookie}; sessionid={session}"),
                    user_agent: String::new(),
                });
            }
        }
    }
    anyhow::bail!("Steam Community cookie missing")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cng_encrypts_and_randomizes_password_padding() {
        let modulus = "ff".repeat(256);
        let first = encrypt(&modulus, "010001", "synthetic-password").unwrap();
        let second = encrypt(&modulus, "010001", "synthetic-password").unwrap();
        assert_eq!(STANDARD.decode(&first).unwrap().len(), 256);
        assert_ne!(first, second);
        assert!(encrypt("not-hex", "010001", "synthetic-password").is_err());
    }
}
