use actix_web::{Responder, get, web};
use serde::Deserialize;
use std::sync::Arc;

use crate::context::CoreCtx;
use crate::error::CoreError;
use crate::http_server::to_response::IntoHttpResponse;
use db::core_schema::{
    CoreDbCrud, DbNewUser, DbNewUserAccount, DbPlatform, DbPlatformMirror, DbUser, DbUserAccount,
    DbVkOauth, DbVkOauthState,
};
use db::error::DbError;

/// Базовый URL для OAuth и обмена токенов в VK.
const VK_OAUTH_URL: &str = "https://oauth.vk.com";

/// Параметры запроса, которые VK отправляет на callback-эндпоинт
/// после успешной авторизации пользователя.
#[derive(Deserialize, Debug)]
pub struct VkCallbackQuery {
    /// Временный код авторизации, который необходимо обменять на токен.
    pub code: String,
    /// Уникальная строка состояния (state), сгенерированная нами при создании ссылки.
    /// Используется для защиты от CSRF-атак и связывания запроса с пользователем.
    pub state: String,
}

/// Структура для десериализации ответа от VK API при обмене
/// authorization code на access_token.
#[derive(Deserialize, Debug)]
struct VkTokenResponse {
    access_token: Option<String>,
    user_id: Option<i64>,
    error: Option<String>,
}

/// Структура для десериализации обёртки ответа от метода VK API `account.getInfo`.
#[derive(Deserialize, Debug)]
struct VkPhoneResponse {
    response: Option<VkPhoneData>,
}

/// Полезная нагрузка ответа VK API, содержащая данные аккаунта,
/// включая номер телефона.
#[derive(Deserialize, Debug)]
struct VkPhoneData {
    phone: Option<String>,
}

/// Основная логика обработки callback-запроса от VK OAuth.
async fn vk_callback_inner(ctx: Arc<CoreCtx>, query: VkCallbackQuery) -> Result<String, CoreError> {
    let pool = ctx.db().get();

    // 1. Получаем state из БД (DbError конвертируется в CoreError автоматически через #[from])
    let state = DbVkOauthState::get_by_state(&query.state, pool).await?;

    // 2. Получаем данные OAuth приложения по project_id
    let oauth = DbVkOauth::get_by_project_id(state.project_id, pool).await?;

    // 3. Получаем base URL для API из platform_mirror
    let mirrors = DbPlatformMirror::get_by_platform_id(state.platform_id, pool).await?;
    let api_base_url = mirrors
        .into_iter()
        .next()
        .map(|m| m.url)
        .ok_or_else(|| "Platform mirror not found".to_string())?; // Конвертируется через From<String>

    // 4. Обмениваем code на access_token
    let client = reqwest::Client::new();
    let redirect_uri = &ctx.cfg().core().vk_redirect_uri;
    let token_url = format!(
        "{}/access_token?client_id={}&client_secret={}&redirect_uri={}&code={}",
        VK_OAUTH_URL,
        oauth.app_id,
        String::from_utf8_lossy(&oauth.secure_key),
        redirect_uri,
        query.code
    );

    // Для внешних библиотек (reqwest) добавляем контекст через map_err + From<String>
    let token_resp = client
        .get(&token_url)
        .send()
        .await
        .map_err(|e| format!("Failed to exchange code: {}", e))?;

    let token_data: VkTokenResponse = token_resp
        .json()
        .await
        .map_err(|e| format!("Invalid token response: {}", e))?;

    if let Some(err) = token_data.error {
        return Err(format!("VK error: {}", err).into());
    }

    let access_token = token_data
        .access_token
        .ok_or_else(|| "No access token in response".to_string())?;

    let vk_user_id = token_data
        .user_id
        .ok_or_else(|| "No user_id in response".to_string())?;

    // 5. Получаем номер телефона через VK API
    let phone_url = format!(
        "{}/method/account.getInfo?user_id={}&fields=phone&access_token={}&v=5.199",
        api_base_url, vk_user_id, access_token
    );

    let phone_resp = client
        .get(&phone_url)
        .send()
        .await
        .map_err(|e| format!("Failed to get phone: {}", e))?;

    let phone_data: VkPhoneResponse = phone_resp
        .json()
        .await
        .map_err(|e| format!("Invalid phone response: {}", e))?;

    let phone = phone_data
        .response
        .and_then(|r| r.phone)
        .ok_or_else(|| "Phone number not provided or error in VK API".to_string())?;

    // 6. Обновляем или создаём пользователя в БД
    let user_account = match DbUserAccount::get_by_external_id(&state.user_ext_id, pool).await {
        Ok(ua) => Some(ua),
        Err(DbError::NotFound { .. }) => None,
        Err(e) => return Err(e.into()), // Явный .into() для DbError внутри match
    };

    let mut user = match DbUser::try_get_by_phone(&phone, pool).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            let new_user = DbNewUser::new(&phone, "VK User");
            new_user.insert(pool).await? // Автоматическая конвертация DbError -> CoreError
        }
        Err(e) => return Err(e.into()),
    };

    if let Some(mut ua) = user_account {
        if ua.user_id != user.pkey() {
            ua.update_user_id(user.pkey(), pool).await?;
        } else if user.phone.is_empty() || user.phone == "unknown" {
            user.update_phone(&phone, pool).await?;
        }
    } else {
        let platform = DbPlatform::get_by_id(state.platform_id, pool).await?;
        let new_ua = DbNewUserAccount::new(&user, &platform, &state.user_ext_id, "VK User");
        new_ua.insert(pool).await?;
    }

    // 7. Удаляем использованный state (ошибка здесь не критична, просто логируем)
    if let Err(e) = DbVkOauthState::delete_by_state(&query.state, pool).await {
        tracing::error!("Error deleting state: {}", e);
    }

    Ok("Phone number verified successfully! You can close this window.".to_string())
}

/// HTTP-эндпоинт для обработки callback-запроса от VK OAuth.
#[utoipa::path(responses((status = 200, body = String)))]
#[get("/vk/callback")]
pub async fn vk_callback(
    data: web::Data<Arc<CoreCtx>>,
    query: web::Query<VkCallbackQuery>,
) -> impl Responder {
    let ctx = data.get_ref().clone();
    let query = query.into_inner();

    vk_callback_inner(ctx, query).await.into_response()
}
