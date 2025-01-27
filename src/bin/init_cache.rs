// src/bin/init_cache.rs
use sqlx::PgPool;
use std::{error::Error, fs};
use dotenv::dotenv;
use std::env;
use serde_json::Value;
use bigdecimal::{BigDecimal, FromPrimitive}; // Import FromPrimitive
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")?;

    // Connect to database
    let pool = PgPool::connect(&database_url).await?;

    // Read initialization data
    let init_data: Value = serde_json::from_str(
        &fs::read_to_string("config/market_init.json")?
    )?;

    // Initialize market cache
    sqlx::query!(
        r#"
        INSERT INTO market_cache (
            sp500_price,
            current_cape,
            cape_period,
            last_yahoo_update,
            last_ycharts_update
        )
        VALUES ($1, $2, $3, $4, $4)
        "#,
        BigDecimal::from(0), // Will be updated by Yahoo fetch
        BigDecimal::from_f64(init_data["cape"]["value"].as_f64().unwrap()).unwrap(), // Use as_f64()
        init_data["cape"]["period"].as_str().unwrap(),
        Utc::now()
    )
    .execute(&pool)
    .await?;

    // Initialize quarterly data
    for (quarter, value) in init_data["quarterly_earnings"].as_object().unwrap() {
        if let Some(num) = value.as_f64() {
            sqlx::query!(
                r#"
                INSERT INTO quarterly_data (
                    quarter,
                    eps_actual,
                    updated_at
                )
                VALUES ($1, $2, $3)
                ON CONFLICT (quarter)
                DO UPDATE SET
                    eps_actual = EXCLUDED.eps_actual,
                    updated_at = EXCLUDED.updated_at
                "#,
                quarter,
                BigDecimal::from_f64(num).unwrap(), // Use from_f64()
                Utc::now()
            )
            .execute(&pool)
            .await?;
        }
    }

    for (quarter, value) in init_data["quarterly_dividends"].as_object().unwrap() {
        if let Some(num) = value.as_f64() {
            sqlx::query!(
                r#"
                INSERT INTO quarterly_data (
                    quarter,
                    dividend,
                    updated_at
                )
                VALUES ($1, $2, $3)
                ON CONFLICT (quarter)
                DO UPDATE SET
                    dividend = EXCLUDED.dividend,
                    updated_at = EXCLUDED.updated_at
                "#,
                quarter,
                BigDecimal::from_f64(num).unwrap(), // Use from_f64()
                Utc::now()
            )
            .execute(&pool)
            .await?;
        }
    }

    for (quarter, value) in init_data["earnings_estimates"].as_object().unwrap() {
        if let Some(num) = value.as_f64() {
            sqlx::query!(
                r#"
                INSERT INTO quarterly_data (
                    quarter,
                    eps_estimated,
                    updated_at
                )
                VALUES ($1, $2, $3)
                ON CONFLICT (quarter)
                DO UPDATE SET
                    eps_estimated = EXCLUDED.eps_estimated,
                    updated_at = EXCLUDED.updated_at
                "#,
                quarter,
                BigDecimal::from_f64(num).unwrap(), // Use from_f64()
                Utc::now()
            )
            .execute(&pool)
            .await?;
        }
    }

    println!("Cache initialization complete!");
    Ok(())
}