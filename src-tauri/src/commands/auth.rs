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
pub async fn get_auth_status(state: State<'_, AppState>) -> Result<AuthStatus, AppError> {
    let db = state.db.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db.lock().unwrap();
        let tokens = token_store::load_tokens(&conn)?;

        let tokens = match tokens {
            Some(t) => t,
            None => {
                return Ok(AuthStatus {
                    logged_in: false,
                    user: None,
                });
            }
        };

        // Proactively validate / refresh the access token on app startup.
        // If the refresh token is genuinely invalid (401), clear the stored
        // tokens so the user sees the login page immediately instead of
        // getting a jarring redirect mid-action later.
        let settings = crate::db::settings::get(&conn)?;
        match client::ensure_fresh_token(&conn, &settings.api_base_url) {
            Ok(Some(t)) => Ok(AuthStatus {
                logged_in: true,
                user: Some(UserInfo {
                    id: t.user_id,
                    email: t.email,
                    display_name: t.display_name,
                }),
            }),
            Ok(None) => Ok(AuthStatus {
                logged_in: false,
                user: None,
            }),
            Err(AppError::Auth(_)) => {
                // Refresh token rejected by server — session is truly expired.
                log::warn!("get_auth_status: session expired, clearing tokens");
                let _ = token_store::clear_tokens(&conn);
                Ok(AuthStatus {
                    logged_in: false,
                    user: None,
                })
            }
            Err(e) => {
                // Network/transient error — don't log the user out.
                // Return logged_in with existing user info; the access token
                // may be stale but ensure_fresh_token already kept old tokens.
                log::warn!("get_auth_status: token check failed (transient): {}", e);
                Ok(AuthStatus {
                    logged_in: true,
                    user: Some(UserInfo {
                        id: tokens.user_id,
                        email: tokens.email,
                        display_name: tokens.display_name,
                    }),
                })
            }
        }
    })
    .await
    .map_err(|e| AppError::General(e.to_string()))?
}
