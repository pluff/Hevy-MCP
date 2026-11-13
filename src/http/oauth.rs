//! Stateless OAuth 2.1 authorization server (PKCE, dynamic client registration).
//!
//! Hevy only supports static API keys, so this server acts as its own authorization
//! server: the user pastes their Hevy API key on the consent page, and every code,
//! access token, refresh token and registered client id is an encrypted
//! (ChaCha20-Poly1305) blob that carries what it needs. Nothing is stored, so tokens
//! survive restarts as long as `OAUTH_SECRET` stays the same.

use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Redirect, Response};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tracing::warn;
use uuid::Uuid;

use super::handler::AppState;

const CODE_TTL: u64 = 300;
const ACCESS_TTL: u64 = 3600;
const REFRESH_TTL: u64 = 90 * 24 * 3600;
const CLIENT_TTL: u64 = 10 * 365 * 24 * 3600;
const NONCE_LEN: usize = 12;

pub(crate) struct OAuth {
    cipher: ChaCha20Poly1305,
    public_url: Option<String>,
    mcp_path: String,
    static_client: Option<(String, String)>,
    static_redirects: Vec<String>,
    http: reqwest::Client,
}

#[derive(Serialize, Deserialize)]
struct ClientBlob {
    exp: u64,
    uris: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    exp: u64,
    key: String,
    #[serde(default)]
    cid: String,
    #[serde(default)]
    ru: String,
    #[serde(default)]
    cc: String,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn oauth_error(status: StatusCode, error: &str, description: &str) -> Response {
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({"error": error, "error_description": description})),
    )
        .into_response()
}

impl OAuth {
    pub(crate) fn from_env(mcp_path: &str) -> Self {
        let secret = env::var("OAUTH_SECRET").unwrap_or_else(|_| {
            warn!(
                "OAUTH_SECRET not set: using a random secret, all OAuth tokens become invalid on restart"
            );
            format!("{}{}", Uuid::new_v4(), Uuid::new_v4())
        });
        let cipher = ChaCha20Poly1305::new(&Sha256::digest(secret.as_bytes()));

        let static_client = match (env::var("OAUTH_CLIENT_ID"), env::var("OAUTH_CLIENT_SECRET")) {
            (Ok(id), Ok(secret)) if !id.is_empty() => Some((id, secret)),
            _ => None,
        };
        let static_redirects = env::var("OAUTH_REDIRECT_URIS")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        Self {
            cipher,
            public_url: env::var("PUBLIC_HOST")
                .ok()
                .map(|h| {
                    let h = h.trim().trim_end_matches('/');
                    let h = h.strip_prefix("https://").unwrap_or(h);
                    format!("https://{h}")
                })
                .filter(|u| u.len() > "https://".len()),
            mcp_path: mcp_path.to_string(),
            static_client,
            static_redirects,
            http: reqwest::Client::new(),
        }
    }

    fn seal<T: Serialize>(&self, typ: &str, value: &T) -> String {
        let plain = serde_json::to_vec(value).unwrap_or_default();
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ct = self
            .cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: &plain,
                    aad: typ.as_bytes(),
                },
            )
            .unwrap_or_default();
        let mut out = nonce.to_vec();
        out.extend(ct);
        URL_SAFE_NO_PAD.encode(out)
    }

    fn open<T: DeserializeOwned>(&self, typ: &str, token: &str) -> Option<T> {
        let data = URL_SAFE_NO_PAD.decode(token).ok()?;
        if data.len() <= NONCE_LEN {
            return None;
        }
        let (nonce, ct) = data.split_at(NONCE_LEN);
        let plain = self
            .cipher
            .decrypt(
                Nonce::from_slice(nonce),
                Payload {
                    msg: ct,
                    aad: typ.as_bytes(),
                },
            )
            .ok()?;
        serde_json::from_slice(&plain).ok()
    }

    fn open_claims(&self, typ: &str, token: &str) -> Option<Claims> {
        self.open::<Claims>(typ, token).filter(|c| c.exp > now())
    }

    pub(crate) fn base_url(&self, headers: &HeaderMap) -> String {
        if let Some(url) = &self.public_url {
            return url.clone();
        }
        let get = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
        let host = get("x-forwarded-host")
            .or_else(|| get("host"))
            .unwrap_or("localhost");
        format!("https://{host}")
    }

    fn resource_url(&self, base: &str) -> String {
        if self.mcp_path == "/" {
            base.to_string()
        } else {
            format!("{base}{}", self.mcp_path)
        }
    }

    /// Validate `Authorization: Bearer`. `None` = no header, `Some(Err)` = invalid token.
    pub(crate) fn bearer_key(&self, headers: &HeaderMap) -> Option<Result<String, ()>> {
        let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
        let token = value
            .strip_prefix("Bearer ")
            .or_else(|| value.strip_prefix("bearer "))?
            .trim();
        Some(
            self.open_claims("access", token)
                .map(|c| c.key)
                .ok_or(()),
        )
    }

    pub(crate) fn unauthorized(&self, headers: &HeaderMap, invalid_token: bool) -> Response {
        let base = self.base_url(headers);
        let suffix = if self.mcp_path == "/" { "" } else { &self.mcp_path };
        let mut challenge = format!(
            "Bearer resource_metadata=\"{base}/.well-known/oauth-protected-resource{suffix}\""
        );
        if invalid_token {
            challenge.push_str(", error=\"invalid_token\"");
        }
        (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, challenge)],
            Json(json!({"error": "unauthorized"})),
        )
            .into_response()
    }

    fn redirect_allowed(&self, client_id: &str, redirect_uri: &str) -> bool {
        if let Some((id, _)) = &self.static_client
            && id == client_id
        {
            return self.static_redirects.is_empty()
                || self.static_redirects.iter().any(|u| u == redirect_uri);
        }
        self.open::<ClientBlob>("client", client_id)
            .is_some_and(|c| c.exp > now() && c.uris.iter().any(|u| u == redirect_uri))
    }

    fn client_authenticated(&self, client_id: &str, secret: Option<&str>) -> bool {
        match &self.static_client {
            Some((id, expected)) if id == client_id => secret == Some(expected.as_str()),
            _ => true,
        }
    }

    async fn key_is_valid(&self, key: &str) -> bool {
        match self
            .http
            .get("https://api.hevyapp.com/v1/workouts/count")
            .header("api-key", key)
            .send()
            .await
        {
            Ok(resp) => !matches!(
                resp.status(),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
            ),
            Err(_) => true,
        }
    }
}

pub(crate) async fn protected_resource(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Value> {
    let base = state.oauth.base_url(&headers);
    Json(json!({
        "resource": state.oauth.resource_url(&base),
        "authorization_servers": [base],
        "bearer_methods_supported": ["header"],
    }))
}

pub(crate) async fn authorization_server(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Json<Value> {
    let base = state.oauth.base_url(&headers);
    Json(json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}/authorize"),
        "token_endpoint": format!("{base}/token"),
        "registration_endpoint": format!("{base}/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none", "client_secret_post", "client_secret_basic"],
    }))
}

pub(crate) async fn register(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let uris: Vec<String> = body
        .get("redirect_uris")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if uris.is_empty() {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "redirect_uris required",
        );
    }
    let client_id = state.oauth.seal(
        "client",
        &ClientBlob {
            exp: now() + CLIENT_TTL,
            uris: uris.clone(),
        },
    );
    (
        StatusCode::CREATED,
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({
            "client_id": client_id,
            "client_name": body.get("client_name").cloned().unwrap_or(Value::Null),
            "redirect_uris": uris,
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none",
        })),
    )
        .into_response()
}

#[derive(Deserialize)]
pub(crate) struct AuthQuery {
    response_type: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    state: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

fn page(client_id: &str, redirect_uri: &str, state: &str, challenge: &str, error: &str) -> Response {
    let error_html = if error.is_empty() {
        String::new()
    } else {
        format!("<p style=\"color:#c00\">{}</p>", esc(error))
    };
    Html(format!(
        r#"<!doctype html><html><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Connect Hevy</title>
<style>body{{font-family:system-ui,sans-serif;max-width:420px;margin:12vh auto;padding:0 16px}}
input{{width:100%;padding:10px;margin:8px 0;box-sizing:border-box}}
button{{padding:10px 16px;width:100%}}</style></head><body>
<h2>Connect Hevy</h2>
<p>Paste your Hevy API key (<a href="https://hevy.com/settings?developer" target="_blank" rel="noopener">hevy.com/settings?developer</a>) to authorize this client.</p>
{error_html}
<form method="post" action="/authorize">
<input type="hidden" name="client_id" value="{client_id}">
<input type="hidden" name="redirect_uri" value="{redirect_uri}">
<input type="hidden" name="state" value="{state}">
<input type="hidden" name="code_challenge" value="{challenge}">
<input type="password" name="api_key" placeholder="Hevy API key" required autofocus>
<button type="submit">Authorize</button>
</form></body></html>"#,
        client_id = esc(client_id),
        redirect_uri = esc(redirect_uri),
        state = esc(state),
        challenge = esc(challenge),
    ))
    .into_response()
}

fn bad_request(msg: &str) -> Response {
    (StatusCode::BAD_REQUEST, msg.to_string()).into_response()
}

pub(crate) async fn authorize_get(
    State(state): State<AppState>,
    Query(q): Query<AuthQuery>,
) -> Response {
    let (Some(client_id), Some(redirect_uri)) = (q.client_id, q.redirect_uri) else {
        return bad_request("client_id and redirect_uri required");
    };
    if !state.oauth.redirect_allowed(&client_id, &redirect_uri) {
        return bad_request("unknown client or redirect_uri");
    }
    if q.response_type.as_deref() != Some("code") {
        return bad_request("response_type must be code");
    }
    let Some(challenge) = q.code_challenge else {
        return bad_request("PKCE code_challenge required");
    };
    if q.code_challenge_method.as_deref().unwrap_or("S256") != "S256" {
        return bad_request("only S256 code_challenge_method supported");
    }
    page(
        &client_id,
        &redirect_uri,
        q.state.as_deref().unwrap_or(""),
        &challenge,
        "",
    )
}

#[derive(Deserialize)]
pub(crate) struct AuthForm {
    client_id: String,
    redirect_uri: String,
    #[serde(default)]
    state: String,
    code_challenge: String,
    api_key: String,
}

pub(crate) async fn authorize_post(
    State(state): State<AppState>,
    Form(f): Form<AuthForm>,
) -> Response {
    if !state.oauth.redirect_allowed(&f.client_id, &f.redirect_uri) {
        return bad_request("unknown client or redirect_uri");
    }
    let key = f.api_key.trim();
    if key.is_empty() || !state.oauth.key_is_valid(key).await {
        return page(
            &f.client_id,
            &f.redirect_uri,
            &f.state,
            &f.code_challenge,
            "Hevy rejected this API key.",
        );
    }
    let code = state.oauth.seal(
        "code",
        &Claims {
            exp: now() + CODE_TTL,
            key: key.to_string(),
            cid: f.client_id.clone(),
            ru: f.redirect_uri.clone(),
            cc: f.code_challenge.clone(),
        },
    );
    let sep = if f.redirect_uri.contains('?') { '&' } else { '?' };
    let mut url = format!("{}{sep}code={code}", f.redirect_uri);
    if !f.state.is_empty() {
        url.push_str(&format!("&state={}", urlencoding::encode(&f.state)));
    }
    Redirect::to(&url).into_response()
}

#[derive(Deserialize)]
pub(crate) struct TokenForm {
    grant_type: String,
    code: Option<String>,
    redirect_uri: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
}

fn basic_credentials(headers: &HeaderMap) -> Option<(String, String)> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let encoded = value.strip_prefix("Basic ")?;
    let decoded = String::from_utf8(STANDARD.decode(encoded.trim()).ok()?).ok()?;
    let (id, secret) = decoded.split_once(':')?;
    Some((
        urlencoding::decode(id).ok()?.into_owned(),
        urlencoding::decode(secret).ok()?.into_owned(),
    ))
}

pub(crate) async fn token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(f): Form<TokenForm>,
) -> Response {
    let oauth = &state.oauth;
    let (client_id, client_secret) = match basic_credentials(&headers) {
        Some((id, secret)) => (Some(id), Some(secret)),
        None => (f.client_id.clone(), f.client_secret.clone()),
    };

    let (key, cid) = match f.grant_type.as_str() {
        "authorization_code" => {
            let Some(claims) = f
                .code
                .as_deref()
                .and_then(|c| oauth.open_claims("code", c))
            else {
                return oauth_error(StatusCode::BAD_REQUEST, "invalid_grant", "invalid or expired code");
            };
            if client_id.as_deref().is_some_and(|id| id != claims.cid)
                || f.redirect_uri.as_deref() != Some(claims.ru.as_str())
            {
                return oauth_error(StatusCode::BAD_REQUEST, "invalid_grant", "client or redirect_uri mismatch");
            }
            let verifier_ok = f.code_verifier.as_deref().is_some_and(|v| {
                URL_SAFE_NO_PAD.encode(Sha256::digest(v.as_bytes())) == claims.cc
            });
            if !verifier_ok {
                return oauth_error(StatusCode::BAD_REQUEST, "invalid_grant", "PKCE verification failed");
            }
            (claims.key, claims.cid)
        }
        "refresh_token" => {
            let Some(claims) = f
                .refresh_token
                .as_deref()
                .and_then(|t| oauth.open_claims("refresh", t))
            else {
                return oauth_error(StatusCode::BAD_REQUEST, "invalid_grant", "invalid or expired refresh_token");
            };
            if client_id.as_deref().is_some_and(|id| id != claims.cid) {
                return oauth_error(StatusCode::BAD_REQUEST, "invalid_grant", "client mismatch");
            }
            (claims.key, claims.cid)
        }
        _ => {
            return oauth_error(
                StatusCode::BAD_REQUEST,
                "unsupported_grant_type",
                "use authorization_code or refresh_token",
            );
        }
    };

    if !oauth.client_authenticated(&cid, client_secret.as_deref()) {
        return oauth_error(StatusCode::UNAUTHORIZED, "invalid_client", "bad client credentials");
    }

    let access = oauth.seal(
        "access",
        &Claims {
            exp: now() + ACCESS_TTL,
            key: key.clone(),
            cid: cid.clone(),
            ru: String::new(),
            cc: String::new(),
        },
    );
    let refresh = oauth.seal(
        "refresh",
        &Claims {
            exp: now() + REFRESH_TTL,
            key,
            cid,
            ru: String::new(),
            cc: String::new(),
        },
    );
    (
        StatusCode::OK,
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({
            "access_token": access,
            "token_type": "Bearer",
            "expires_in": ACCESS_TTL,
            "refresh_token": refresh,
        })),
    )
        .into_response()
}
