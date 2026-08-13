use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Asset {
    pub id: i64,
    pub name: String,
    pub unit_value: f64,
}

pub struct UserRecord {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

#[derive(Serialize, Clone)]
pub struct AssetPurchase {
    pub id: i64,
    pub transaction_type: String,
    pub created_at: String,
    pub quantity: f64,
    pub purchase_price: f64,
    pub pnl: f64,
}

#[derive(Serialize)]
pub struct UserAsset {
    pub asset_id: i64,
    pub name: String,
    pub quantity: f64,
    pub unit_value: f64,
    pub purchase_price: f64,
    pub current_value: f64,
    pub pnl: f64,
    pub pnl_percent: f64,
    pub purchases: Vec<AssetPurchase>,
}