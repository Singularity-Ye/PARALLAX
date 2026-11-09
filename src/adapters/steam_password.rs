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

pub async fn save_password(account_id: &str, username: &str, password: &str) -> anyhow::Result<()> {
    let account_id = account_id.to_owned();
    let username = username.to_owned();
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || {
        let raw = serde_json::to_vec(&(username, password))?;
        keyring::Entry::new("Parallax.steam_passwords.v1", &account_id)?
            .set_secret(&raw)
            .map_err(|_| anyhow::anyhow!("Windows 凭据管理器保存密码失败"))
    })
    .await?
}

pub async fn load_password(account_id: &str) -> anyhow::Result<Option<(String, String)>> {
    let account_id = account_id.to_owned();
    tokio::task::spawn_blocking(move || {
        match keyring::Entry::new("Parallax.steam_passwords.v1", &account_id)?.get_secret() {
            Ok(bytes) => {
                let (u, p): (String, String) = serde_json::from_slice(&bytes)?;
                Ok(Some((u, p)))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => anyhow::bail!("读取已存 Steam 密码失败"),
        }
    })
    .await?
}

pub async fn remove_password(account_id: &str) -> anyhow::Result<()> {
    let account_id = account_id.to_owned();
    tokio::task::spawn_blocking(move || {
        match keyring::Entry::new("Parallax.steam_passwords.v1", &account_id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => anyhow::bail!("清理已存 Steam 密码失败"),
        }
    })
    .await?
}

/// Smart Steam RSA + Guard 2FA / Mobile Confirmation login:
/// - Auto-discovers 17-digit SteamID if `expected_steam_id` is empty or `"00000000000000000"`.
/// - Supports `shared_secret` auto-code, manual 5-digit Guard code, or 45s mobile app confirmation polling (with `new_client_id` rotation).
/// - Returns `(Credentials, discovered_steam_id)`.
pub async fn login_smart<F: std::future::Future<Output = Option<String>> + Send>(
    username: &str,
    password: &str,
    expected_steam_id: &str,
    manual_guard_code: Option<&str>,
    guard: impl FnOnce(u64) -> F,
) -> anyhow::Result<(Credentials, String)> {
    const API: &str = "https://api.steampowered.com/IAuthenticationService";
    const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";
    let client = super::network::client_builder()
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(UA)
        .timeout(Duration::from_secs(15))
        .build()?;
    let rsa: Value = client
        .get(format!("{API}/GetPasswordRSAPublicKey/v1"))
        .query(&[("account_name", username)])
        .send()
        .await
        .context("无法连接 Steam 认证服务器（获取 RSA 公钥失败），请检查代理或本机网络")?
        .error_for_status()
        .context("Steam 认证服务器拒绝 RSA 公钥请求")?
        .json()
        .await?;
    let rsa = &rsa["response"];
    let encrypted = encrypt(
        field(rsa, "publickey_mod")?,
        field(rsa, "publickey_exp")?,
        password,
    )?;
    let begin_resp = post(
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
    .await
    .context("提交 Steam 账号密码失败，请检查网络连接")?;

    let eresult = begin_resp
        .headers()
        .get("x-eresult")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("1")
        .to_owned();
    let begin: Value = begin_resp.json().await?;
    let begin = &begin["response"];

    let actual_steam_id = begin["steamid"].as_str().unwrap_or_default().trim().to_owned();
    if actual_steam_id.is_empty() || eresult == "5" {
        anyhow::bail!("Steam 账号名或密码错误，请核对后重新输入（EResult={eresult}）");
    }
    if eresult == "84" || eresult == "87" {
        anyhow::bail!("Steam 登录请求过于频繁触发短期限制，请稍等几分钟后再试");
    }
    let expected = expected_steam_id.trim();
    if !expected.is_empty() && expected != "00000000000000000" && actual_steam_id != expected {
        anyhow::bail!(
            "该登录账号对应的 SteamID ({actual_steam_id}) 与当前卡片登记的 SteamID ({expected}) 不一致"
        );
    }

    let mut current_client_id = field(begin, "client_id")?.to_owned();
    let request_id = field(begin, "request_id")?;
    let empty_vec = Vec::new();
    let allowed = begin["allowed_confirmations"]
        .as_array()
        .unwrap_or(&empty_vec);

    // Steam EAuthSessionGuardType: 1=None, 2=EmailCode, 3=DeviceCode, 4=DeviceConfirmation(Mobile Push), 5=EmailConfirmation
    let has_device_code = allowed.iter().any(|v| v["confirmation_type"] == 3);
    let has_email_code = allowed.iter().any(|v| v["confirmation_type"] == 2);
    let has_mobile_conf = allowed.iter().any(|v| v["confirmation_type"] == 4);
    let has_none = allowed.iter().any(|v| v["confirmation_type"] == 1) || allowed.is_empty();

    let manual_code = manual_guard_code.map(str::trim).filter(|s| !s.is_empty());
    let auto_code = if manual_code.is_none() {
        // Query authoritative Steam server time so local clock drift never breaks TOTP!
        let mut steam_time = crate::domain::selection::now();
        if let Ok(resp) = post(
            &client,
            "https://api.steampowered.com/ITwoFactorService/QueryTime/v0001",
            &[("steamid", "0")],
        )
        .await
        {
            if let Ok(time_json) = resp.json::<Value>().await {
                if let Some(st) = time_json["response"]["server_time"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .or_else(|| time_json["response"]["server_time"].as_u64())
                {
                    steam_time = st;
                }
            }
        }
        guard(steam_time).await
    } else {
        None
    };
    let chosen_code = manual_code.map(str::to_owned).or(auto_code);
    let mut code_accepted = false;
    let mut auto_code_rejected = false;

    if (has_device_code || has_email_code) && chosen_code.is_some() {
        let code = chosen_code.as_deref().unwrap();
        let code_type = if has_device_code { "3" } else { "2" };
        let response = post(
            &client,
            &format!("{API}/UpdateAuthSessionWithSteamGuardCode/v1"),
            &[
                ("client_id", &current_client_id),
                ("steamid", &actual_steam_id),
                ("code_type", code_type),
                ("code", code),
            ],
        )
        .await?;
        let ok_eresult = response
            .headers()
            .get("x-eresult")
            .is_none_or(|value| value == "1");
        if ok_eresult {
            code_accepted = true;
        } else if manual_code.is_some() || !has_mobile_conf {
            anyhow::bail!("Steam 令牌验证码被拒绝：请检查手动输入的 5 位令牌码或「Steam 令牌」页配置的 shared_secret");
        } else {
            // Local shared_secret did not match this account's phone authenticator,
            // but Steam Mobile App push confirmation (type 4) is active — fall through to poll!
            auto_code_rejected = true;
        }
    } else if !has_none && !has_mobile_conf {
        anyhow::bail!(
            "该账号开启了二次验证，但尚未配置 shared_secret。请前往「Steam 令牌」页绑定密钥，或在登录框填写当前 5 位验证码"
        );
    }

    // Poll so mobile app confirmation has time, handling new_client_id rotation
    let mut token = None;
    let max_polls = if code_accepted || has_none { 8 } else { 35 };
    for _ in 0..max_polls {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let poll: Value = post(
            &client,
            &format!("{API}/PollAuthSessionStatus/v1"),
            &[("client_id", &current_client_id), ("request_id", request_id)],
        )
        .await?
        .json()
        .await?;
        let resp_obj = &poll["response"];
        if let Some(value) = resp_obj["refresh_token"]
            .as_str()
            .filter(|s| !s.is_empty())
        {
            token = Some(value.to_owned());
            break;
        }
        if let Some(new_cid) = resp_obj["new_client_id"].as_str().filter(|s| !s.is_empty()) {
            current_client_id = new_cid.to_owned();
        }
    }
    let timeout_msg = if auto_code_rejected {
        "本地配置的 shared_secret 与该 Steam 账号手机令牌不匹配（自动算码未通过），且等待手机 App 批准超时。请在登录框填写手机 App 上的 5 位验证码，或在手机 App 中点击批准登录"
    } else {
        "Steam 登录等待确认超时：请在 Steam 手机 App 中点击批准登录，或在登录框填写 5 位验证码"
    };
    let token = token.context(timeout_msg)?;
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
    let finalized_sid = field(&finalized, "steamID")?;
    ensure!(
        finalized_sid == actual_steam_id,
        "Steam 令牌交换返回的身份不一致"
    );
    let transfers = finalized["transfer_info"]
        .as_array()
        .context("Steam 未返回 Cookie 交换列表")?;
    let mut community_cookie: Option<String> = None;
    for transfer in transfers {
        let Ok(url) = reqwest::Url::parse(transfer["url"].as_str().unwrap_or_default()) else {
            continue;
        };
        if url.scheme() != "https" || url.path() != "/login/settoken" {
            continue;
        }
        let host = url.host_str().unwrap_or_default();
        if host != "steamcommunity.com" && host != "store.steampowered.com" {
            continue;
        }
        let params = &transfer["params"];
        let ( Some(nonce), Some(auth) ) = (params["nonce"].as_str(), params["auth"].as_str()) else {
            continue;
        };
        if let Ok(response) = post(
            &client,
            url.as_str(),
            &[
                ("steamID", &actual_steam_id),
                ("nonce", nonce),
                ("auth", auth),
            ],
        )
        .await
        {
            if host == "steamcommunity.com" {
                for header in response.headers().get_all("set-cookie") {
                    if let Some(cookie) = header
                        .to_str()
                        .ok()
                        .and_then(|v| v.split(';').next())
                        .filter(|v| v.starts_with("steamLoginSecure="))
                    {
                        community_cookie = Some(cookie.to_owned());
                    }
                }
            }
        }
    }
    let cookie = community_cookie.context("未能从 Steam 获取 steamLoginSecure 会话凭据")?;
    Ok((
        Credentials {
            cookies: format!("{cookie}; sessionid={session}"),
            user_agent: UA.into(),
        },
        actual_steam_id,
    ))
}

fn currency_info(currency_id: u32) -> Option<(&'static str, &'static str, &'static str, bool)> {
    // Returns (code, symbol, default_region, no_divide)
    Some(match currency_id {
        1 => ("USD", "$", "US", false),
        2 => ("GBP", "£", "GB", false),
        3 => ("EUR", "€", "EU", false),
        5 => ("RUB", "₽", "RU", false),
        6 => ("PLN", "zł", "PL", false),
        7 => ("BRL", "R$", "BR", false),
        8 => ("JPY", "¥", "JP", true),
        9 => ("NOK", "kr", "NO", false),
        10 => ("IDR", "Rp", "ID", false),
        11 => ("MYR", "RM", "MY", false),
        12 => ("PHP", "₱", "PH", false),
        13 => ("SGD", "S$", "SG", false),
        14 => ("THB", "฿", "TH", false),
        15 => ("VND", "₫", "VN", true),
        16 => ("KRW", "₩", "KR", true),
        17 => ("TRY", "₺", "TR", false),
        18 => ("UAH", "₴", "UA", false),
        19 => ("MXN", "Mex$", "MX", false),
        20 => ("CAD", "C$", "CA", false),
        21 => ("AUD", "A$", "AU", false),
        22 => ("NZD", "NZ$", "NZ", false),
        23 => ("CNY", "¥", "CN", false),
        24 => ("INR", "₹", "IN", false),
        25 => ("CLP", "CLP$", "CL", false),
        26 => ("PEN", "S/.", "PE", false),
        27 => ("COP", "COL$", "CO", false),
        28 => ("ZAR", "R", "ZA", false),
        29 => ("HKD", "HK$", "HK", false),
        30 => ("TWD", "NT$", "TW", false),
        31 => ("SAR", "SR", "SA", false),
        32 => ("AED", "AED", "AE", false),
        34 => ("ARS", "ARS$", "AR", false),
        35 => ("ILS", "₪", "IL", false),
        37 => ("KZT", "₸", "KZ", false),
        38 => ("KWD", "KD", "KW", false),
        39 => ("QAR", "QR", "QA", false),
        _ => return None,
    })
}

/// Fetches Steam avatar, persona_name, wallet balance, settlement currency, and region code
/// using the active proxy/network settings and the account's Steam cookies.
pub async fn fetch_profile_and_wallet(
    steam_id: &str,
    cookies: Option<&str>,
) -> crate::domain::accounts::AccountProfile {
    let mut out = crate::domain::accounts::AccountProfile::default();
    let steam_id = steam_id.trim();
    if steam_id.len() != 17 || steam_id == "00000000000000000" {
        return out;
    }
    const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";
    let client = super::network::client_builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    // 1. Fetch Persona Name & Avatar via Steam miniprofile JSON API
    if let Some(acc_id_32) = steam_id
        .parse::<u64>()
        .ok()
        .and_then(|sid| sid.checked_sub(76561197960265728))
    {
        let url = format!("https://steamcommunity.com/miniprofile/{acc_id_32}/json");
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(data) = resp.json::<Value>().await {
                    if let Some(name) = data["persona_name"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
                        out.display_name = name
                            .replace("&gt;", ">")
                            .replace("&lt;", "<")
                            .replace("&amp;", "&")
                            .replace("&quot;", "\"")
                            .replace("&#39;", "'");
                    }
                    if let Some(av) = data["avatar_url"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
                        let mut full = if av.starts_with("http") {
                            av.to_owned()
                        } else {
                            format!("https://avatars.steamstatic.com/{av}")
                        };
                        full = full.replace("_medium", "_full");
                        out.avatar_url = full;
                    }
                }
            }
        }
    }

    // Fallback: XML profile
    if out.display_name.is_empty() || out.avatar_url.is_empty() {
        let url = format!("https://steamcommunity.com/profiles/{steam_id}?xml=1");
        if let Ok(resp) = client.get(&url).send().await {
            if let Ok(text) = resp.text().await {
                if out.display_name.is_empty() {
                    if let Some(name) = text
                        .split_once("<steamID><![CDATA[")
                        .and_then(|(_, s)| s.split_once("]]></steamID>"))
                        .map(|(n, _)| n.trim())
                        .filter(|s| !s.is_empty())
                    {
                        out.display_name = name.to_owned();
                    }
                }
                if out.avatar_url.is_empty() {
                    if let Some(av) = text
                        .split_once("<avatarFull><![CDATA[")
                        .and_then(|(_, s)| s.split_once("]]></avatarFull>"))
                        .map(|(a, _)| a.trim())
                        .filter(|s| !s.is_empty())
                    {
                        out.avatar_url = av.to_owned();
                    }
                }
            }
        }
    }

    // 2. If cookies are present, fetch Wallet Balance, Settlement Currency & Region from steamcommunity.com/market/
    if let Some(cookie_str) = cookies.map(str::trim).filter(|s| s.contains("steamLoginSecure=")) {
        if let Ok(resp) = client
            .get("https://steamcommunity.com/market/")
            .header("Cookie", cookie_str)
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .send()
            .await
        {
            if let Ok(html) = resp.text().await {
                if let Some((_, after)) = html.split_once("g_rgWalletInfo") {
                    if let Some(start) = after.find('{') {
                        let rest = &after[start..];
                        if let Some(end) = rest.find('}') {
                            let json_slice = &rest[..=end];
                            if let Ok(wallet) = serde_json::from_str::<Value>(json_slice) {
                                let bal = wallet["wallet_balance"]
                                    .as_i64()
                                    .or_else(|| wallet["wallet_balance"].as_str()?.parse().ok())
                                    .unwrap_or(0);
                                let delayed = wallet["wallet_delayed_balance"]
                                    .as_i64()
                                    .or_else(|| wallet["wallet_delayed_balance"].as_str()?.parse().ok())
                                    .unwrap_or(0);
                                let total = bal + delayed;
                                let cid = wallet["wallet_currency"]
                                    .as_u64()
                                    .or_else(|| wallet["wallet_currency"].as_str()?.parse().ok())
                                    .unwrap_or(0) as u32;
                                let country = wallet["wallet_country"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .trim()
                                    .to_uppercase();
                                if let Some((code, sym, def_region, no_div)) = currency_info(cid) {
                                    out.currency_code = code.to_owned();
                                    out.wallet_currency_id = Some(cid);
                                    out.wallet_balance_raw = Some(total);
                                    out.region_code = if country.len() == 2 {
                                        country
                                    } else {
                                        def_region.to_owned()
                                    };
                                    out.wallet_balance_display = if no_div {
                                        format!("{sym} {total}")
                                    } else {
                                        format!("{sym} {:.2}", total as f64 / 100.0)
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    out
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
