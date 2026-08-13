-- Add up migration script here
CREATE TABLE IF NOT EXISTS user_assets (
    id BIGSERIAL PRIMARY KEY NOT NULL,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    asset_id BIGINT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    purchase_price DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    quantity DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    UNIQUE (user_id, asset_id)
);