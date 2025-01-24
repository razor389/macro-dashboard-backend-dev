// src/bin/setup_db.rs
use sqlx::PgPool;
use std::{error::Error, str::FromStr};
use dotenv::dotenv;
use std::env;
use csv::Reader;
use std::fs::File;
use bigdecimal::BigDecimal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    
    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")?;
    
    // Connect to database
    let pool = PgPool::connect(&database_url).await?;
    
    // Create tables if they don't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS market_cache (
            id SERIAL PRIMARY KEY,
            sp500_price DECIMAL NOT NULL,
            current_cape DECIMAL NOT NULL,
            last_yahoo_update TIMESTAMP WITH TIME ZONE NOT NULL,
            last_ycharts_update TIMESTAMP WITH TIME ZONE NOT NULL
        );

        CREATE TABLE IF NOT EXISTS quarterly_data (
            quarter VARCHAR(6) PRIMARY KEY,
            ttm_dividend DECIMAL,
            eps_actual DECIMAL,
            eps_estimated DECIMAL,
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL
        );

        CREATE TABLE IF NOT EXISTS historical_data (
            year INTEGER PRIMARY KEY,
            sp500_price DECIMAL NOT NULL,
            dividend DECIMAL NOT NULL,
            eps DECIMAL NOT NULL,
            cape DECIMAL NOT NULL
        );
        "#
    )
    .execute(&pool)
    .await?;

    // Read CSV file
    let file = File::open("stk_mkt.csv")?;
    let mut rdr = Reader::from_reader(file);
    
    // Insert data from CSV
    for result in rdr.records() {
        let record = result?;
        
        if &record[0] == "Year" {
            continue;
        }
        
        let year: i32 = record[0].trim().parse()?;
        let sp500_price = BigDecimal::from_str(record[1].trim())?;
        let dividend = BigDecimal::from_str(record[2].trim())?;
        let eps = BigDecimal::from_str(record[3].trim())?;
        let cape = BigDecimal::from_str(record[4].trim())?;
        
        sqlx::query!(
            r#"
            INSERT INTO historical_data (year, sp500_price, dividend, eps, cape)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (year) DO UPDATE SET
                sp500_price = EXCLUDED.sp500_price,
                dividend = EXCLUDED.dividend,
                eps = EXCLUDED.eps,
                cape = EXCLUDED.cape
            "#,
            year,
            sp500_price,
            dividend,
            eps,
            cape
        )
        .execute(&pool)
        .await?;
    }
    
    println!("Database setup complete!");
    Ok(())
}