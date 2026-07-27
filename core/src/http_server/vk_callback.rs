use crate::context::CoreCtx;
use actix_web::{HttpResponse, Responder, get, web};
use db::core_schema::{
    CoreDbCrud, DbNewUser, DbNewUserAccount, DbPlatform, DbUser, DbUserAccount, DbVkOauth,
    DbVkOauthState,
};
use db::error::DbError;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct VkCallbackQuery {
    pub code: String,
    pub state: String,
}

#[get("/vk/callback")]
pub async fn vk_callback(
    data: web::Data<Arc<CoreCtx>>,
    query: web::Query<VkCallbackQuery>,
) -> impl Responder {
    let ctx = data.get_ref();
    let pool = ctx.db().get();

    // 1. Получаем state из БД
    let state = match DbVkOauthState::get_by_state(&query.state, pool).await {
        Ok(s) => s,
        Err(DbError::NotFound { .. }) => {
            return HttpResponse::BadRequest().body("Invalid or expired state");
        }
        Err(e) => {
            tracing::error!("DB error getting state: {}", e);
            return HttpResponse::InternalServerError().body("Internal error");
        }
    };

    // 2. Получаем данные OAuth приложения
    let oauth = match DbVkOauth::get_by_platform_id(state.platform_id, pool).await {
        Ok(o) => o,
        Err(_) => return HttpResponse::InternalServerError().body("OAuth config not found"),
    };

    // 3. Обмениваем code на access_token
    let client = reqwest::Client::new();
    // Получаем redirect_uri из конфигурации
    let redirect_uri = &ctx.cfg().core().vk_redirect_uri;
    let token_url = format!(
        "https://oauth.vk.com/access_token?client_id={}&client_secret={}&redirect_uri={}&code={}",
        oauth.app_id,
        String::from_utf8_lossy(&oauth.secure_key),
        redirect_uri,
        query.code
    );

    let token_resp = match client.get(&token_url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Error exchanging code: {}", e);
            return HttpResponse::InternalServerError().body("Failed to exchange code");
        }
    };

    #[derive(Deserialize)]
    struct VkTokenResponse {
        access_token: Option<String>,
        user_id: Option<i64>,
        error: Option<String>,
    }

    let token_data: VkTokenResponse = match token_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Error parsing token response: {}", e);
            return HttpResponse::InternalServerError().body("Invalid token response");
        }
    };

    if let Some(err) = token_data.error {
        return HttpResponse::BadRequest().body(format!("VK error: {}", err));
    }

    let access_token = match token_data.access_token {
        Some(t) => t,
        None => return HttpResponse::InternalServerError().body("No access token in response"),
    };

    let vk_user_id = match token_data.user_id {
        Some(id) => id,
        None => return HttpResponse::InternalServerError().body("No user_id in response"),
    };

    // 4. Получаем номер телефона через VK API
    let phone_url = format!(
        "https://api.vk.com/method/account.getInfo?user_id={}&fields=phone&access_token={}&v=5.199",
        vk_user_id, access_token
    );

    let phone_resp = match client.get(&phone_url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Error getting phone: {}", e);
            return HttpResponse::InternalServerError().body("Failed to get phone");
        }
    };

    #[derive(Deserialize)]
    struct VkPhoneResponse {
        response: Option<VkPhoneData>,
        error: Option<serde_json::Value>,
    }

    #[derive(Deserialize)]
    struct VkPhoneData {
        phone: Option<String>,
    }

    let phone_data: VkPhoneResponse = match phone_resp.json().await {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Error parsing phone response: {}", e);
            return HttpResponse::InternalServerError().body("Invalid phone response");
        }
    };

    let phone = match phone_data.response.and_then(|r| r.phone) {
        Some(p) => p,
        None => {
            return HttpResponse::BadRequest().body("Phone number not provided or error in VK API");
        }
    };

    // 5. Обновляем или создаём пользователя в БД
    let user_account = match DbUserAccount::get_by_external_id(&state.user_ext_id, pool).await {
        Ok(ua) => Some(ua),
        Err(DbError::NotFound { .. }) => None,
        Err(e) => {
            tracing::error!("DB error getting user account: {}", e);
            return HttpResponse::InternalServerError().body("Internal error");
        }
    };

    let user = match DbUser::try_get_by_phone(&phone, pool).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            // Создаём нового пользователя
            let new_user = DbNewUser::new(&phone, "VK User");
            match new_user.insert(pool).await {
                Ok(u) => u,
                Err(e) => {
                    tracing::error!("Error creating user: {}", e);
                    return HttpResponse::InternalServerError().body("Failed to create user");
                }
            }
        }
        Err(e) => {
            tracing::error!("DB error getting user by phone: {}", e);
            return HttpResponse::InternalServerError().body("Internal error");
        }
    };

    if let Some(ua) = user_account {
        // Если учётная запись уже существует, проверяем привязку
        if ua.user_id != user.pkey() {
            // Перепривязываем учётную запись к пользователю с этим телефоном
            if let Err(e) = ua.update_user_id(user.pkey(), pool).await {
                tracing::error!("Error re-linking user account: {}", e);
                return HttpResponse::InternalServerError().body("Failed to update user account");
            }
        } else if user.phone.is_empty() || user.phone == "unknown" {
            // Обновляем телефон, если он был пустым
            if let Err(e) = user.update_phone(&phone, pool).await {
                tracing::error!("Error updating user phone: {}", e);
                return HttpResponse::InternalServerError().body("Failed to update user phone");
            }
        }
    } else {
        // Создаём новую учётную запись
        let platform = match DbPlatform::get_by_id(state.platform_id, pool).await {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Error getting platform: {}", e);
                return HttpResponse::InternalServerError().body("Internal error");
            }
        };
        let new_ua = DbNewUserAccount::new(&user, &platform, &state.user_ext_id, "VK User");
        if let Err(e) = new_ua.insert(pool).await {
            tracing::error!("Error creating user account: {}", e);
            return HttpResponse::InternalServerError().body("Failed to create user account");
        }
    }

    // 6. Удаляем использованный state
    if let Err(e) = DbVkOauthState::delete_by_state(&query.state, pool).await {
        tracing::error!("Error deleting state: {}", e);
    }

    HttpResponse::Ok().body("Phone number verified successfully! You can close this window.")
}
