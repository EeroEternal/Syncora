use tauri::State;
use serde::Serialize;
use crate::auth::{client, token_store::{self, AuthTokens}};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct AuthStatus {
    pub logged_in: bool,
    pub user: Option<UserInfo>,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
}

#[tauri::command]
pub async fn register(
    state: State<'_, AppState>,
    email: String,
    password: String,
) -> Result<UserInfo, AppError> {
    let db = state.db.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock().unwrap();
        let settings = crate::db::settings::get(&conn)?;

        let resp = client::register(&settings.api_base_url, &email, &password)?;

        let tokens = AuthTokens {
            access_token: resp.access_token,
            refresh_token: resp.refresh_token,
            user_id: resp.user.id.clone(),
            email: resp.user.email.clone(),
            display_name: resp.user.display_name.clone(),
        };
        token_store::save_tokens(&conn, &tokens)?;

        Ok(UserInfo {
            id: resp.user.id,
            email: resp.user.email,
            display_name: resp.user.display_name,
        })
    })
    .await
    .map_err(|e| AppError::General(e.to_string()))?
}

#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    email: String,
    password: String,
) -> Result<UserInfo, AppError> {
    let db = state.db.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock().unwrap();
        let settings = crate::db::settings::get(&conn)?;

        let resp = client::login(&settings.api_base_url, &email, &password)?;

        let tokens = AuthTokens {
            access_token: resp.access_token,
            refresh_token: resp.refresh_token,
            user_id: resp.user.id.clone(),
            email: resp.user.email.clone(),
            display_name: resp.user.display_name.clone(),
        };
        token_store::save_tokens(&conn, &tokens)?;

        Ok(UserInfo {
            id: resp.user.id,
            email: resp.user.email,
            display_name: resp.user.display_name,
        })
    })
    .await
    .map_err(|e| AppError::General(e.to_string()))?
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), AppError> {
    let db = state.db.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock().unwrap();
        let settings = crate::db::settings::get(&conn)?;

        if let Ok(Some(tokens)) = token_store::load_tokens(&conn) {
            let _ = client::logout(
                &settings.api_base_url,
                &tokens.access_token,
                &tokens.refresh_token,
            );
        }

        token_store::clear_tokens(&conn)?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::General(e.to_string()))?
}

#[tauri::command]
pub fn get_auth_status(state: State<AppState>) -> Result<AuthStatus, AppError> {
    let conn = state.db.lock().unwrap();
    let tokens = token_store::load_tokens(&conn)?;

    match tokens {
        Some(t) => Ok(AuthStatus {
            logged_in: true,
            user: Some(UserInfo {
                id: t.user_id,
                email: t.email,
                display_name: t.display_name,
            }),
        }),
        None => Ok(AuthStatus {
            logged_in: false,
            user: None,
        }),
    }
}
