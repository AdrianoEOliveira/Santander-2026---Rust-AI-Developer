use axum::{Json, Router, routing::get};
use serde::Deserialize;

use crate::{
    app::AppState, auth::admin::Admin, error::AppError, models::Asset, repository::Repository,
};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/assets",
        get(list_assets).post(create_asset).patch(update_asset),
    )
}

#[tracing::instrument(skip_all)]
async fn list_assets(repostiory: Repository) -> Result<Json<Vec<Asset>>, AppError> {
    let assets = repostiory.list_assets().await?;
    Ok(Json(assets))
}

#[derive(Deserialize)]
struct CreateAssetRequest {
    name: String,
    unit_value: f64,
}

#[tracing::instrument(skip_all)]
async fn create_asset(
    _: Admin,
    repostiory: Repository,
    Json(request): Json<CreateAssetRequest>,
) -> Result<Json<Asset>, AppError> {
    let new_asset = repostiory
        .create_asset(request.name, request.unit_value)
        .await?;

    Ok(Json(new_asset))
}

#[derive(Deserialize)]
struct UpdateAssetRequest {
    id: i64,
    name: Option<String>,
    unit_value: Option<f64>,
}

#[tracing::instrument(skip_all)]
async fn update_asset(
    _: Admin,
    repostiory: Repository,
    Json(request): Json<UpdateAssetRequest>,
) -> Result<Json<Asset>, AppError> {
    match repostiory
        .update_asset(request.id, request.name, request.unit_value)
        .await?
    {
        Some(updated_asset) => Ok(Json(updated_asset)),
        None => Err(AppError::AssetDoesNotExist),
    }
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn test_create_asset(db: PgPool) {
        let request = CreateAssetRequest {
            name: "Bitcoin".to_string(),
            unit_value: 10.0,
        };
        let Json(new_asset) = create_asset(Admin, db.into(), Json(request))
            .await
            .expect("success");

        assert_eq!(new_asset.id, 1);
        assert_eq!(new_asset.name, "Bitcoin");
        assert_eq!(new_asset.unit_value, 10.0);

        insta::assert_json_snapshot!(new_asset);
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_list_assets(db: PgPool) {
        let Json(assets) = list_assets(db.into()).await.expect("success");

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name, "Bitcoin");

        insta::assert_json_snapshot!(assets);
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_update_asset(db: PgPool) {
        let request = UpdateAssetRequest {
            id: 1,
            name: Some("Ethereum".to_string()),
            unit_value: Some(20.0),
        };

        let Json(updated_asset) = update_asset(Admin, db.into(), Json(request))
            .await
            .expect("success");

        assert_eq!(updated_asset.id, 1);
        assert_eq!(updated_asset.name, "Ethereum");
        assert_eq!(updated_asset.unit_value, 20.0);

        insta::assert_json_snapshot!(updated_asset);
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_update_asset_name_only(db: PgPool) {
        let request = UpdateAssetRequest {
            id: 1,
            name: Some("Bitcoin Updated".to_string()),
            unit_value: None,
        };

        let Json(updated_asset) = update_asset(Admin, db.into(), Json(request))
            .await
            .expect("success");

        assert_eq!(updated_asset.id, 1);
        assert_eq!(updated_asset.name, "Bitcoin Updated");

        // O valor original deve ser preservado
        assert_eq!(updated_asset.unit_value, 10.0);

        insta::assert_json_snapshot!(updated_asset);
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_update_asset_value_only(db: PgPool) {
        let request = UpdateAssetRequest {
            id: 1,
            name: None,
            unit_value: Some(25.0),
        };

        let Json(updated_asset) = update_asset(Admin, db.into(), Json(request))
            .await
            .expect("success");

        assert_eq!(updated_asset.id, 1);

        assert_eq!(updated_asset.name, "Bitcoin");

        assert_eq!(updated_asset.unit_value, 25.0);

        insta::assert_json_snapshot!(updated_asset);
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_user_assets_workflow(db: PgPool) {
        let repo = Repository::from(db);

        let user = repo
            .add_user("John", "hashed_password")
            .await
            .expect("create user");

        let assets = repo
            .list_user_assets(user.id)
            .await
            .expect("list initial assets");

        assert!(assets.is_empty());

        repo.buy_user_asset(user.id, 1, 2.5, None)
            .await
            .expect("buy asset");

        let user_assets = repo.list_user_assets(user.id).await.expect("list assets");

        assert_eq!(user_assets.len(), 1);
        assert_eq!(user_assets[0].name, "Bitcoin");
        assert_eq!(user_assets[0].quantity, 2.5);
        assert_eq!(user_assets[0].unit_value, 10.0);

        repo.buy_user_asset(user.id, 1, 1.5, None)
            .await
            .expect("buy more asset");

        let updated_assets = repo
            .list_user_assets(user.id)
            .await
            .expect("list updated assets");

        assert_eq!(updated_assets.len(), 1);
        assert_eq!(updated_assets[0].quantity, 4.0);
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_multiple_users_have_independent_assets(db: PgPool) {
        let repo = Repository::from(db);

        let user1 = repo
            .add_user("John", "hashed_password")
            .await
            .expect("create user 1");

        let user2 = repo
            .add_user("Marie", "hashed_password")
            .await
            .expect("create user 2");

        repo.buy_user_asset(user1.id, 1, 5.0, None)
            .await
            .expect("user 1 buy");

        repo.buy_user_asset(user2.id, 1, 2.0, None)
            .await
            .expect("user 2 buy");

        let user1_assets = repo
            .list_user_assets(user1.id)
            .await
            .expect("list user 1 assets");

        assert_eq!(user1_assets.len(), 1);
        assert_eq!(user1_assets[0].quantity, 5.0);

        let user2_assets = repo
            .list_user_assets(user2.id)
            .await
            .expect("list user 2 assets");

        assert_eq!(user2_assets.len(), 1);
        assert_eq!(user2_assets[0].quantity, 2.0);
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_buy_multiple_assets_same_user(db: PgPool) {
        let repo = Repository::from(db);

        let user = repo
            .add_user("John", "hashed_password")
            .await
            .expect("create user");

        repo.buy_user_asset(user.id, 1, 2.0, None)
            .await
            .expect("buy bitcoin");
        let assets = repo.list_user_assets(user.id).await.expect("list assets");

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name, "Bitcoin");
        assert_eq!(assets[0].quantity, 2.0);
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_user_can_buy_same_asset_multiple_times(db: PgPool) {
        let repo = Repository::from(db);

        let user = repo
            .add_user("John", "hashed_password")
            .await
            .expect("create user");

        repo.buy_user_asset(user.id, 1, 1.0, None)
            .await
            .expect("first buy");

        repo.buy_user_asset(user.id, 1, 2.0, None)
            .await
            .expect("second buy");

        repo.buy_user_asset(user.id, 1, 3.0, None)
            .await
            .expect("third buy");

        let assets = repo.list_user_assets(user.id).await.expect("list assets");

        assert_eq!(assets.len(), 1);

        assert_eq!(assets[0].quantity, 6.0);
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_buy_asset_with_custom_unit_value(db: PgPool) {
        let repo = Repository::from(db);

        let user = repo
            .add_user("John", "hashed_password")
            .await
            .expect("create user");

        // Buy 2 Bitcoins at $5.0 custom price (Market base price is $10.0)
        repo.buy_user_asset(user.id, 1, 2.0, Some(5.0))
            .await
            .expect("buy bitcoin with custom price");

        let assets = repo.list_user_assets(user.id).await.expect("list assets");

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name, "Bitcoin");
        assert_eq!(assets[0].quantity, 2.0);
        assert_eq!(assets[0].unit_value, 10.0); // Market base price remains 10.0
        assert_eq!(assets[0].purchase_price, 5.0);
        assert_eq!(assets[0].pnl, 10.0); // (10.0 - 5.0) * 2.0 = +10.0 Profit!
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_sell_asset_profit_and_loss(db: PgPool) {
        let repo = Repository::from(db);

        let user = repo
            .add_user("John", "hashed_password")
            .await
            .expect("create user");

        // Buy 10 Bitcoins at $5.0 each (Market base price is $10.0) -> Lucro: +$5.0 per unit * 10 = +$50.0
        repo.buy_user_asset(user.id, 1, 10.0, Some(5.0))
            .await
            .expect("buy asset");

        let assets = repo.list_user_assets(user.id).await.expect("list assets");
        assert_eq!(assets[0].quantity, 10.0);
        assert_eq!(assets[0].unit_value, 10.0);
        assert_eq!(assets[0].purchase_price, 5.0);
        assert_eq!(assets[0].pnl, 50.0); // (10 - 5) * 10 = +50.0

        // Sell 4 Bitcoins
        repo.sell_user_asset(user.id, 1, 4.0, Some(10.0))
            .await
            .expect("sell asset");

        let assets_after_sell = repo.list_user_assets(user.id).await.expect("list assets");
        assert_eq!(assets_after_sell[0].quantity, 6.0);
        assert_eq!(assets_after_sell[0].unit_value, 10.0);
        assert_eq!(assets_after_sell[0].pnl, 50.0); // 30.0 unrealized + 20.0 realized = 50.0 total cumulative PnL

        // Sell remaining 6 Bitcoins
        repo.sell_user_asset(user.id, 1, 6.0, None)
            .await
            .expect("sell remaining asset");

        let assets_final = repo.list_user_assets(user.id).await.expect("list assets");
        assert_eq!(assets_final.len(), 1);
        assert_eq!(assets_final[0].quantity, 0.0);
    }

    #[sqlx::test]
    async fn test_admin_update_asset_base_price_for_future_transactions(db: PgPool) {
        let repo = Repository::from(db);
        repo.ensure_default_assets().await.unwrap();

        let user = repo
            .add_user("testadminuser", "hash")
            .await
            .expect("add user");

        // User buys 2 Bitcoins at $10.0 initial price
        repo.buy_user_asset(user.id, 1, 2.0, Some(10.0))
            .await
            .expect("buy asset");

        let assets_initial = repo.list_user_assets(user.id).await.expect("list assets");
        assert_eq!(assets_initial[0].unit_value, 10.0);
        assert_eq!(assets_initial[0].purchase_price, 10.0);

        // Admin updates Bitcoin base market price to $50.0
        repo.update_asset(1, None, Some(50.0))
            .await
            .expect("update asset")
            .expect("asset exists");

        // List assets should now reflect the new base market unit value of $50.0
        let assets_after_update = repo.list_user_assets(user.id).await.expect("list assets");
        assert_eq!(assets_after_update[0].unit_value, 50.0);
        // Past buy purchase_price remains $10.0
        assert_eq!(assets_after_update[0].purchases[0].purchase_price, 10.0);
        // PnL recalculated against new base price: (50 - 10) * 2 = +80.0
        assert_eq!(assets_after_update[0].pnl, 80.0);
    }
}