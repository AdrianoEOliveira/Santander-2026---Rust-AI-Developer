use std::convert::Infallible;

use axum::extract::FromRequestParts;
use sqlx::PgPool;

use crate::{
    app::AppState,
    models::{Asset, AssetPurchase, UserAsset, UserRecord},
};

pub struct Repository {
    db: PgPool,
}

impl Repository {
    pub async fn ensure_default_admin(&self) -> sqlx::Result<()> {
        let admin_user = self.get_user_by_name("admin").await?;
        if admin_user.is_none() {
            let password_hash = password_auth::generate_hash("admin");
            let _ = self.add_user("admin", &password_hash).await?;
        }
        Ok(())
    }

    pub async fn ensure_default_assets(&self) -> sqlx::Result<()> {
        let defaults = [
            ("Bitcoin", 10.0),
            ("Ethereum", 20.0),
            ("Dólar", 5.5),
            ("Real", 1.0),
        ];

        for (name, value) in defaults {
            sqlx::query!(
                "INSERT INTO assets (name, unit_value)
                 VALUES ($1, $2)
                 ON CONFLICT (name) DO NOTHING;",
                name,
                value
            )
            .execute(&self.db)
            .await?;
        }

        self.ensure_default_admin().await?;

        Ok(())
    }

    pub async fn list_assets(&self) -> sqlx::Result<Vec<Asset>> {
        sqlx::query_as!(
            Asset,
            "SELECT id, name, unit_value
             FROM assets;"
        )
        .fetch_all(&self.db)
        .await
    }

    pub async fn create_asset(&self, name: String, unit_value: f64) -> sqlx::Result<Asset> {
        sqlx::query_as!(
            Asset,
            "INSERT INTO assets (name, unit_value)
             VALUES ($1, $2)
             RETURNING id, name, unit_value;",
            name,
            unit_value
        )
        .fetch_one(&self.db)
        .await
    }

    pub async fn update_asset(
        &self,
        asset_id: i64,
        name: Option<String>,
        unit_value: Option<f64>,
    ) -> sqlx::Result<Option<Asset>> {
        sqlx::query_as!(
            Asset,
            "UPDATE assets
             SET name=COALESCE($2, name),
                 unit_value=COALESCE($3, unit_value)
             WHERE id=$1
             RETURNING id, name, unit_value;",
            asset_id,
            name,
            unit_value
        )
        .fetch_optional(&self.db)
        .await
    }

    pub async fn add_user(&self, username: &str, password_hash: &str) -> sqlx::Result<UserRecord> {
        sqlx::query_as!(
            UserRecord,
            "INSERT INTO users (username, password_hash)
             VALUES ($1, $2)
             RETURNING id, username, password_hash;",
            username,
            password_hash,
        )
        .fetch_one(&self.db)
        .await
    }

    pub async fn get_user_by_name(&self, username: &str) -> sqlx::Result<Option<UserRecord>> {
        sqlx::query_as!(
            UserRecord,
            "SELECT id, username, password_hash
             FROM users
             WHERE username = $1;",
            username
        )
        .fetch_optional(&self.db)
        .await
    }

    pub async fn list_user_assets(&self, user_id: i64) -> sqlx::Result<Vec<UserAsset>> {
        struct UserAssetDbRow {
            asset_id: i64,
            name: String,
            quantity: f64,
            unit_value: f64,
            purchase_price: f64,
            current_value: f64,
        }

        let base_assets = sqlx::query_as!(
            UserAssetDbRow,
            "SELECT 
                ua.asset_id, 
                a.name, 
                ua.quantity, 
                a.unit_value,
                ua.purchase_price,
                (ua.quantity * a.unit_value) as \"current_value!\"
             FROM user_assets ua
             JOIN assets a ON ua.asset_id = a.id
             WHERE ua.user_id = $1;",
            user_id
        )
        .fetch_all(&self.db)
        .await?;

        let mut result = Vec::new();

        for item in base_assets {
            struct TxRow {
                id: i64,
                transaction_type: String,
                quantity: f64,
                purchase_price: f64,
                created_at: String,
            }

            let txs = sqlx::query_as!(
                TxRow,
                "SELECT id, transaction_type, quantity, purchase_price, to_char(created_at, 'YYYY-MM-DD HH24:MI') as \"created_at!\"
                 FROM user_transactions
                 WHERE user_id = $1 AND asset_id = $2
                 ORDER BY created_at ASC;",
                user_id,
                item.asset_id
            )
            .fetch_all(&self.db)
            .await?;

            let (total_pnl, total_cost, purchases) = if !txs.is_empty() {
                let mut sum_pnl = 0.0;
                let mut sum_cost = 0.0;
                let list = txs
                    .into_iter()
                    .map(|tx| {
                        let tx_pnl = if tx.transaction_type == "sell" {
                            (tx.purchase_price - item.purchase_price) * tx.quantity
                        } else {
                            (item.unit_value - tx.purchase_price) * tx.quantity
                        };

                        sum_pnl += tx_pnl;

                        if tx.transaction_type == "buy" {
                            sum_cost += tx.quantity * tx.purchase_price;
                        }
                        AssetPurchase {
                            id: tx.id,
                            transaction_type: tx.transaction_type,
                            created_at: tx.created_at,
                            quantity: tx.quantity,
                            purchase_price: tx.purchase_price,
                            pnl: tx_pnl,
                        }
                    })
                    .collect();
                (sum_pnl, sum_cost, list)
            } else {
                let cost = item.quantity * item.purchase_price;
                let fallback_pnl = (item.unit_value - item.purchase_price) * item.quantity;
                (
                    fallback_pnl,
                    cost,
                    vec![AssetPurchase {
                        id: 0,
                        transaction_type: "buy".to_string(),
                        created_at: "2026-03-20 10:25".to_string(),
                        quantity: item.quantity,
                        purchase_price: item.purchase_price,
                        pnl: fallback_pnl,
                    }],
                )
            };

            let avg_purchase_price = item.purchase_price;

            let pnl_percent = if total_cost > 0.0 {
                (total_pnl / total_cost) * 100.0
            } else if avg_purchase_price > 0.0 {
                ((item.unit_value - avg_purchase_price) / avg_purchase_price) * 100.0
            } else {
                0.0
            };

            result.push(UserAsset {
                asset_id: item.asset_id,
                name: item.name,
                quantity: item.quantity,
                unit_value: item.unit_value,
                purchase_price: avg_purchase_price,
                current_value: item.current_value,
                pnl: total_pnl,
                pnl_percent,
                purchases,
            });
        }

        Ok(result)
    }

    pub async fn buy_user_asset(
        &self,
        user_id: i64,
        asset_id: i64,
        quantity: f64,
        unit_value: Option<f64>,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO user_transactions (user_id, asset_id, quantity, purchase_price, transaction_type)
             VALUES ($1, $2, $3, COALESCE($4, (SELECT unit_value FROM assets WHERE id = $2)), 'buy');",
            user_id,
            asset_id,
            quantity,
            unit_value
        )
        .execute(&self.db)
        .await?;

        sqlx::query!(
            "INSERT INTO user_assets (user_id, asset_id, quantity, purchase_price)
             VALUES ($1, $2, $3, COALESCE($4, (SELECT unit_value FROM assets WHERE id = $2)))
             ON CONFLICT (user_id, asset_id)
             DO UPDATE SET 
                purchase_price = ((user_assets.quantity * user_assets.purchase_price) + (EXCLUDED.quantity * EXCLUDED.purchase_price)) / (user_assets.quantity + EXCLUDED.quantity),
                quantity = user_assets.quantity + EXCLUDED.quantity;",
            user_id,
            asset_id,
            quantity,
            unit_value
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn sell_user_asset(
        &self,
        user_id: i64,
        asset_id: i64,
        quantity: f64,
        sell_price: Option<f64>,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO user_transactions (user_id, asset_id, quantity, purchase_price, transaction_type)
             VALUES ($1, $2, $3, COALESCE($4, (SELECT unit_value FROM assets WHERE id = $2)), 'sell');",
            user_id,
            asset_id,
            quantity,
            sell_price
        )
        .execute(&self.db)
        .await?;

        let mut remaining_to_sell = quantity;
        struct TxItem {
            id: i64,
            quantity: f64,
        }
        let buy_txs = sqlx::query_as!(
            TxItem,
            "SELECT id, quantity FROM user_transactions
             WHERE user_id = $1 AND asset_id = $2 AND transaction_type = 'buy' AND quantity > 0
             ORDER BY created_at ASC;",
            user_id,
            asset_id
        )
        .fetch_all(&self.db)
        .await?;

        for tx in buy_txs {
            if remaining_to_sell <= 0.0 {
                break;
            }
            if tx.quantity <= remaining_to_sell {
                remaining_to_sell -= tx.quantity;
                sqlx::query!(
                    "UPDATE user_transactions SET quantity = 0 WHERE id = $1;",
                    tx.id
                )
                .execute(&self.db)
                .await?;
            } else {
                let new_qty = tx.quantity - remaining_to_sell;
                remaining_to_sell = 0.0;
                sqlx::query!(
                    "UPDATE user_transactions SET quantity = $2 WHERE id = $1;",
                    tx.id,
                    new_qty
                )
                .execute(&self.db)
                .await?;
            }
        }

        sqlx::query!(
            "UPDATE user_assets
             SET quantity = GREATEST(0.0, user_assets.quantity - $3)
             WHERE user_id = $1 AND asset_id = $2 AND quantity >= $3;",
            user_id,
            asset_id,
            quantity
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

impl FromRequestParts<AppState> for Repository {
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self {
            db: state.db.clone(),
        })
    }
}

#[cfg(test)]
impl From<PgPool> for Repository {
    fn from(db: PgPool) -> Self {
        Self { db }
    }
}
