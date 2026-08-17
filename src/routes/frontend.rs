use askama::Template;
use axum::{
    Form, Router,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use serde::Deserialize;

use crate::{
    app::AppState,
    auth::user::{UnauthenticatedUser, User},
    error::AppError,
    models::{Asset, UserAsset},
    repository::Repository,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/login", get(login_page).post(login))
        .route("/register", get(register_page).post(register))
        .route("/logout", axum::routing::post(logout))
        .route("/Carteira/buy", axum::routing::post(buy_asset))
        .route("/Carteira/sell", axum::routing::post(sell_asset))
        .route("/assets/new", axum::routing::post(create_asset))
        .route("/assets/update_price", axum::routing::post(update_asset_price))
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage;

async fn login_page() -> Result<Html<String>, AppError> {
    let html = LoginPage.render()?;
    Ok(Html(html))
}

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterPage;

async fn register_page() -> Result<Html<String>, AppError> {
    let html = RegisterPage.render()?;
    Ok(Html(html))
}

#[derive(Deserialize)]
struct RegisterForm {
    username: String,
    password: String,
}

async fn register(
    repository: Repository,
    Form(request): Form<RegisterForm>,
) -> Result<impl IntoResponse, AppError> {
    let unauth_user = UnauthenticatedUser::new(request.username, request.password);
    unauth_user.register(&repository).await?;

    Ok(Redirect::to("/login"))
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn login(
    repository: Repository,
    jar: CookieJar,
    Form(request): Form<LoginForm>,
) -> Result<impl IntoResponse, AppError> {
    let unauth_user = UnauthenticatedUser::new(request.username, request.password);
    let user = unauth_user.authenticate(&repository).await?;

    let token = user.auth_token()?;
    let cookie = Cookie::build(("token", token)).path("/").http_only(true);

    Ok((jar.add(cookie), Redirect::to("/")))
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardPage {
    username: String,
    user_assets: Vec<UserAsset>,
    assets: Vec<Asset>,
}

async fn index(maybe_user: Option<User>, repository: Repository) -> Result<Response, AppError> {
    let user = match maybe_user {
        Some(user) => user,
        None => return Ok(Redirect::to("/login").into_response()),
    };

    repository.ensure_default_assets().await?;
    let user_assets = repository.list_user_assets(user.id()).await?;
    let assets = repository.list_assets().await?;

    let page = DashboardPage {
        username: user.username().clone(),
        user_assets,
        assets,
    };

    let html = page.render()?;
    Ok(Html(html).into_response())
}

#[derive(Deserialize)]
struct BuyAssetForm {
    asset_id: i64,
    quantity: f64,
    unit_value: Option<f64>,
}

async fn buy_asset(
    maybe_user: Option<User>,
    repository: Repository,
    Form(form): Form<BuyAssetForm>,
) -> Result<impl IntoResponse, AppError> {
    let user = match maybe_user {
        Some(user) => user,
        None => return Err(AppError::MissingAuthorization),
    };

    if form.quantity <= 0.0 {
        return Err(AppError::InvalidQuantity);
    }
    if let Some(uv) = form.unit_value {
        if uv <= 0.0 {
            return Err(AppError::InvalidUnitPrice);
        }
    }

    repository
        .buy_user_asset(user.id(), form.asset_id, form.quantity, form.unit_value)
        .await?;

    Ok(Redirect::to("/"))
}

#[derive(Deserialize)]
struct SellAssetForm {
    asset_id: i64,
    quantity: f64,
    sell_price: Option<f64>,
}

async fn sell_asset(
    maybe_user: Option<User>,
    repository: Repository,
    Form(form): Form<SellAssetForm>,
) -> Result<impl IntoResponse, AppError> {
    let user = match maybe_user {
        Some(user) => user,
        None => return Err(AppError::MissingAuthorization),
    };

    if form.quantity <= 0.0 {
        return Err(AppError::InvalidQuantity);
    }
    if let Some(sp) = form.sell_price {
        if sp <= 0.0 {
            return Err(AppError::InvalidUnitPrice);
        }
    }

    repository
        .sell_user_asset(user.id(), form.asset_id, form.quantity, form.sell_price)
        .await?;

    Ok(Redirect::to("/"))
}

#[derive(Deserialize)]
struct CreateAssetForm {
    name: String,
    unit_value: f64,
}

async fn create_asset(
    maybe_user: Option<User>,
    repository: Repository,
    Form(form): Form<CreateAssetForm>,
) -> Result<impl IntoResponse, AppError> {
    let _user = match maybe_user {
        Some(user) => user,
        None => return Err(AppError::MissingAuthorization),
    };

    repository.create_asset(form.name, form.unit_value).await?;

    Ok(Redirect::to("/"))
}

#[derive(Deserialize)]
struct UpdateAssetPriceForm {
    asset_id: i64,
    unit_value: f64,
}

async fn update_asset_price(
    maybe_user: Option<User>,
    repository: Repository,
    Form(form): Form<UpdateAssetPriceForm>,
) -> Result<impl IntoResponse, AppError> {
    let user = match maybe_user {
        Some(user) => user,
        None => return Err(AppError::MissingAuthorization),
    };

    if user.username() != "admin" {
        return Err(AppError::Unauthorized);
    }

    if form.unit_value <= 0.0 {
        return Err(AppError::InvalidUnitPrice);
    }

    repository
        .update_asset(form.asset_id, None, Some(form.unit_value))
        .await?;

    Ok(Redirect::to("/"))
}

async fn logout(jar: CookieJar) -> impl IntoResponse {
    let cookie = Cookie::build(("token", "")).path("/").http_only(true);
    (jar.remove(cookie), Redirect::to("/login"))
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn test_unregistered_user_cannot_login(db: PgPool) {
        let repo = Repository::from(db);
        let jar = CookieJar::new();

        let form = LoginForm {
            username: "nonexistent".to_string(),
            password: "password123".to_string(),
        };

        let result = login(repo, jar, Form(form)).await;
        assert!(matches!(result, Err(AppError::UserDoesNotExist)));
    }

    #[sqlx::test]
    async fn test_register_and_login_flow(db: PgPool) {
        let repo = Repository::from(db.clone());

        let reg_form = RegisterForm {
            username: "newuser".to_string(),
            password: "password123".to_string(),
        };

        let response = register(repo, Form(reg_form)).await;
        assert!(response.is_ok());

        let login_repo = Repository::from(db);
        let jar = CookieJar::new();
        let login_form = LoginForm {
            username: "newuser".to_string(),
            password: "password123".to_string(),
        };

        let login_res = login(login_repo, jar, Form(login_form)).await;
        assert!(login_res.is_ok());
    }
}
