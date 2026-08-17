//! Внутренний функционал проверок.
//! Проверка JWT работает на двух уровнях
use actix_web::HttpRequest;
use actix_web::dev::ServiceRequest;
use actix_web::http::header::{HeaderName, HeaderValue};
use base64::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::UzorPluginConfig;
use crate::error::{Result, UzorPluginError};

/// Хедер JWT токена.
/// Теоретический, мы должны его парзить и выбирать механизм расшифровки
/// контента. При этом разрабатывать расшфровку нескольки разных механизмом расшифровки верификации
/// для нас излишне.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct TokenHeader {
    alg: String,
    typ: String,
    kid: String,
}

/// Расшифрованный javetta token.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TokenData {
    pub personal_id: String,
    pub role: String,
}

/// Базовая сущность ASAA-вского проекта.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AsaaProject {
    pub client_digital_code: u32,
    pub name: String,
}

/// Данные которые мы получаем из ASAA.
/// Нам интересно только поле
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AsaaData {
    pub status_code: u16,
    pub status: String,
    pub projects: Vec<AsaaProject>,
    pub themes: Vec<String>,
    pub pins: Vec<String>,
    pub skillgroups: Vec<String>,
    pub calltypes: Vec<String>,
    pub campaigns: Vec<String>,
    pub is_advances: String,
}

/// Ответ из Keycloak:
///```ignore
/// {
///   "permissions": [
///     {
///       "resource_id": "90ccc6fc-b296-4cd1-881e-089e1ee15957",
///       "resource_name": "Hello World Resource"
///     }
///   ],
///   "exp": 1465314139,
///   "nbf": 0,
///   "iat": 1465313839,
///   "aud": "hello-world-authz-service",
///   "active": true
/// }
/// ```
///
/// Negative res
/// ```ignore
/// {
///   "active": false
/// }
/// ```
///
///  По факту нам этот ответ не нужно подробно обрабатывать.
/// Нам нужно лишь поле `active`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct KeyCloakIntrospectResponse {
    pub active: bool,
    pub error: Option<String>,
}

pub trait PermissionReExtract: for<'de> Deserialize<'de> {
    const HEADER_KEY: &str;

    /// Достать данные токена, которые МЫ подставляем в запрос из запроса.
    fn from_final_request(req: &HttpRequest) -> Result<Self> {
        let key = HeaderName::from_static(Self::HEADER_KEY);
        let Some(token) = req.headers().get(key) else {
            return Err(crate::error::UzorPluginError::MissingHeader(
                Self::HEADER_KEY,
            ));
        };
        Self::decode(token.to_str().unwrap_or("Invalid"))
    }

    /// Получить данные токена из хедера.
    fn decode(header: &str) -> Result<Self> {
        let decoded_data = BASE64_STANDARD.decode(header)?;
        let data = serde_json::from_slice(&decoded_data)?;

        Ok(data)
    }
}

impl PermissionReExtract for TokenData {
    const HEADER_KEY: &str = "core-token-data";
}

impl PermissionReExtract for AsaaData {
    const HEADER_KEY: &str = "core-permitted-projects";
}

/// Послать в Keycloak запрос о валидности токена.
// curl
//  -u "client_id:client_secret"
//  -d "token=access_token_to_introspect"
//  "http://$KC_SERVER/$KC_CONTEXT/realms/$REALM/protocol/openid-connect/token/introspect"
// curl --location --request POST '${keycloakBaseUrl}/realms/${keycloakRealm}/protocol/openid-connect/token/introspect' \
// --header 'Authorization: Basic insert_auth_code_here' \
// --header 'Content-Type: application/x-www-form-urlencoded' \
// --data-urlencode 'token=insert_refresh_token_here' \
// --data-urlencode 'token_type_hint=refresh_token'
#[tracing::instrument(skip_all)]
pub(crate) async fn check_token_via_keycloak_server(
    req: &HttpRequest,
    cfg: &UzorPluginConfig,
    client: &Client,
) -> Result<TokenData> {
    tracing::info!("Checking token via keycloak server");

    let Some(token) = req.headers().get("Authorization") else {
        return Err(UzorPluginError::KeycloakTokenMissing);
    };
    let token = token.to_str()?.replace("Bearer ", "");
    let path = format!(
        "{host}/realms/{realm}/protocol/openid-connect/token/introspect",
        host = cfg.keycloak.home,
        realm = cfg.keycloak.realm,
    );
    let data = [("token", token.to_string())];
    let result = client
        .post(path)
        .basic_auth(&cfg.keycloak.client_id, cfg.keycloak.client_secret.as_ref())
        .form(&data.into_iter().collect::<HashMap<_, _>>())
        .send()
        .await?
        .text()
        .await?;

    let result: KeyCloakIntrospectResponse = serde_json::from_str(&result)?;

    if let Some(err) = result.error {
        Err(UzorPluginError::KeycloakGeneral(err))
    } else if result.active {
        parse_token(token)
    } else {
        Err(UzorPluginError::KeycloakTokenExpired)
    }
}

/// Проверить токен по алгоритму проверки данных против шифрованного.
pub(crate) fn check_token_via_sha512(
    req: &HttpRequest,
    cfg: &UzorPluginConfig,
) -> Result<TokenData> {
    use josekit::{jws, jwt};

    tracing::info!("Checking token via sha-512");

    let Some(token) = req.headers().get("Authorization") else {
        return Err(UzorPluginError::KeycloakTokenMissing);
    };
    let token = token.to_str()?.replace("Bearer ", "");
    let (header, data) = get_parts(&token)?;

    let decoded_header = BASE64_STANDARD.decode(header)?;
    let header: TokenHeader = serde_json::from_slice(&decoded_header)?;

    // Проверь ччто хедер правильного типа.
    if &header.typ != "JWT" {
        return Err(UzorPluginError::KeycloakTokenInvalid);
    }
    let secret = &cfg.keycloak.public_key;
    let key = format!("-----BEGIN PUBLIC KEY-----\n{secret}\n-----END PUBLIC KEY-----");

    match &header.alg as &str {
        "RS256" => jwt::decode_with_verifier(&token, &jws::RS256.verifier_from_pem(&key)?)?,
        "RS384" => jwt::decode_with_verifier(&token, &jws::RS384.verifier_from_pem(&key)?)?,
        "RS512" => jwt::decode_with_verifier(&token, &jws::RS512.verifier_from_pem(&key)?)?,
        "PS256" => jwt::decode_with_verifier(&token, &jws::PS256.verifier_from_pem(&key)?)?,
        "PS384" => jwt::decode_with_verifier(&token, &jws::PS384.verifier_from_pem(&key)?)?,
        "PS512" => jwt::decode_with_verifier(&token, &jws::PS512.verifier_from_pem(&key)?)?,
        "ES256" => jwt::decode_with_verifier(&token, &jws::ES256.verifier_from_pem(&key)?)?,
        "ES384" => jwt::decode_with_verifier(&token, &jws::ES384.verifier_from_pem(&key)?)?,
        "ES512" => jwt::decode_with_verifier(&token, &jws::ES512.verifier_from_pem(&key)?)?,
        x => return Err(UzorPluginError::KeycloakUnsupportedSign(x.to_string())),
    };

    let decoded_data = BASE64_STANDARD.decode(data)?;
    let data = serde_json::from_slice(&decoded_data)?;
    Ok(data)
}

fn pad(v: &str) -> String {
    let mut v = v.to_owned();
    while !v.len().is_multiple_of(4) {
        v.push('=');
    }
    v
}

#[tracing::instrument]
pub(crate) fn get_parts(token: &str) -> Result<(String, String)> {
    let mut splits = token.split('.');

    let Some(header) = splits.next() else {
        return Err(UzorPluginError::KeycloakTokenInvalid);
    };
    let Some(data) = splits.next() else {
        return Err(UzorPluginError::KeycloakTokenInvalid);
    };
    Ok((pad(header), pad(data)))
}

/// Распарзить JWT токен.
#[tracing::instrument]
pub(crate) fn parse_token(token: String) -> Result<TokenData> {
    let (_, data) = get_parts(&token)?;

    let decoded_data = BASE64_STANDARD.decode(data)?;
    let data = serde_json::from_slice(&decoded_data)?;

    Ok(data)
}

/// Распарзить JWT токен.
#[tracing::instrument(skip_all)]
pub(crate) async fn get_asaa_data(
    token: &TokenData,
    cfg: &UzorPluginConfig,
    client: &Client,
) -> Result<AsaaData> {
    tracing::info!("Checking ASAA roles");
    let path = format!(
        "{host}/v1/employee/whats?personalId={id}",
        host = cfg.asaa_home,
        id = token.personal_id,
    );
    let res: AsaaData = client.post(path).send().await?.json().await?;

    Ok(res)
}

pub(crate) fn fill_request(
    req: &mut ServiceRequest,
    token: TokenData,
    data: AsaaData,
) -> Result<()> {
    let projects = serde_json::to_string(&data)?;
    let encoded_projects = BASE64_STANDARD.encode(&projects);

    let token = serde_json::to_string(&token)?;
    let encoded_token_data = BASE64_STANDARD.encode(&token);

    let thn = HeaderName::from_static("core-token-data");
    let phn = HeaderName::from_static("core-permitted-projects");

    let tvn = HeaderValue::from_str(&encoded_token_data)?;
    let pvn = HeaderValue::from_str(&encoded_projects)?;

    req.headers_mut().insert(thn, tvn);
    req.headers_mut().insert(phn, pvn);

    Ok(())
}
