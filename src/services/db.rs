// src/services/db.rs
use sqlx::PgPool;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::error::Error;
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use serde::{Serialize, Deserialize};

#[derive(Debug, Default, Serialize, Deserialize)] 
pub struct CacheTimestamps {
    #[serde(with = "chrono::serde::ts_seconds")]
    pub yahoo_price: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub ycharts_data: DateTime<Utc>,
}

#[derive(Debug, Default)]
pub struct MarketCache {
    pub timestamps: CacheTimestamps,
    pub sp500_price: f64,
    pub current_cape: f64,
    pub cape_period: String,
    pub quarterly_dividends: HashMap<String, f64>,
    pub eps_actual: HashMap<String, f64>,
    pub eps_estimated: HashMap<String, f64>,
}

#[derive(Debug, Serialize)]
pub struct HistoricalRecord {
    pub year: i32,
    pub sp500_price: f64,
    pub dividend: f64,
    pub eps: f64,
    pub cape: f64,
}

pub struct DbStore {
    pub(crate) pool: PgPool
}

impl DbStore {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error>> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn get_market_cache(&self) -> Result<MarketCache, Box<dyn Error>> {
        let cache = sqlx::query!(
            r#"
            SELECT sp500_price, current_cape, cape_period, last_yahoo_update, last_ycharts_update 
            FROM market_cache 
            ORDER BY id DESC LIMIT 1
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        let quarters = sqlx::query!(
            "SELECT quarter, dividend, eps_actual, eps_estimated FROM quarterly_data"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut quarterly_dividends = HashMap::new();
        let mut eps_actual = HashMap::new();
        let mut eps_estimated = HashMap::new();

        for q in quarters {
            if let Some(div) = q.dividend {
                quarterly_dividends.insert(q.quarter.clone(), div.to_f64().unwrap_or(0.0));
            }
            if let Some(eps) = q.eps_actual {
                eps_actual.insert(q.quarter.clone(), eps.to_f64().unwrap_or(0.0));
            }
            if let Some(eps) = q.eps_estimated {
                eps_estimated.insert(q.quarter.clone(), eps.to_f64().unwrap_or(0.0));
            }
        }

        if let Some(cache) = cache {
            Ok(MarketCache {
                timestamps: CacheTimestamps {
                    yahoo_price: cache.last_yahoo_update,
                    ycharts_data: cache.last_ycharts_update,
                },
                sp500_price: cache.sp500_price.to_f64().unwrap_or(0.0),
                current_cape: cache.current_cape.to_f64().unwrap_or(0.0),
                cape_period: cache.cape_period,
                quarterly_dividends,
                eps_actual,
                eps_estimated,
            })
        } else {
            Ok(MarketCache::default())
        }
    }

    pub async fn update_market_cache(&self, cache: &MarketCache) -> Result<(), Box<dyn Error>> {
        let mut tx = self.pool.begin().await?;

        // Convert f64 to BigDecimal for the market cache
        let sp500_price = BigDecimal::from_f64(cache.sp500_price)
            .ok_or("Failed to convert sp500_price to BigDecimal")?;
        let current_cape = BigDecimal::from_f64(cache.current_cape)
            .ok_or("Failed to convert current_cape to BigDecimal")?;

        // Update main cache
        sqlx::query!(
            r#"
            INSERT INTO market_cache (sp500_price, current_cape, cape_period, last_yahoo_update, last_ycharts_update)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            sp500_price,
            current_cape,
            cache.cape_period,
            cache.timestamps.yahoo_price,
            cache.timestamps.ycharts_data,
        )
        .execute(&mut *tx)
        .await?;

        // Update quarterly data
        for (quarter, div) in &cache.quarterly_dividends {
            let div_decimal = BigDecimal::from_f64(*div)
                .ok_or("Failed to convert dividend to BigDecimal")?;
            
            let eps_actual = cache.eps_actual.get(quarter)
                .map(|v| BigDecimal::from_f64(*v)
                    .ok_or("Failed to convert eps_actual to BigDecimal"))
                .transpose()?;
            
            let eps_estimated = cache.eps_estimated.get(quarter)
                .map(|v| BigDecimal::from_f64(*v)
                    .ok_or("Failed to convert eps_estimated to BigDecimal"))
                .transpose()?;
            
            sqlx::query!(
                r#"
                INSERT INTO quarterly_data (quarter, dividend, eps_actual, eps_estimated, updated_at)
                VALUES ($1, $2, $3, $4, NOW())
                ON CONFLICT (quarter) 
                DO UPDATE SET 
                    dividend = EXCLUDED.dividend,
                    eps_actual = CASE 
                        WHEN EXCLUDED.eps_actual IS NOT NULL THEN EXCLUDED.eps_actual 
                        ELSE quarterly_data.eps_actual 
                    END,
                    eps_estimated = CASE 
                        WHEN EXCLUDED.eps_estimated IS NOT NULL THEN EXCLUDED.eps_estimated 
                        ELSE quarterly_data.eps_estimated 
                    END,
                    updated_at = NOW()
                "#,
                quarter,
                div_decimal,
                eps_actual,
                eps_estimated,
            )
            .execute(&mut *tx)
            .await?;
        }

        // Update standalone EPS entries
        for (quarter, eps) in &cache.eps_actual {
            if !cache.quarterly_dividends.contains_key(quarter) {
                let eps_decimal = BigDecimal::from_f64(*eps)
                    .ok_or("Failed to convert eps to BigDecimal")?;
                
                sqlx::query!(
                    r#"
                    INSERT INTO quarterly_data (quarter, eps_actual, updated_at)
                    VALUES ($1, $2, NOW())
                    ON CONFLICT (quarter) 
                    DO UPDATE SET 
                        eps_actual = EXCLUDED.eps_actual,
                        updated_at = NOW()
                    "#,
                    quarter,
                    eps_decimal,
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        for (quarter, eps) in &cache.eps_estimated {
            if !cache.quarterly_dividends.contains_key(quarter) {
                let eps_decimal = BigDecimal::from_f64(*eps)
                    .ok_or("Failed to convert eps to BigDecimal")?;
                
                sqlx::query!(
                    r#"
                    INSERT INTO quarterly_data (quarter, eps_estimated, updated_at)
                    VALUES ($1, $2, NOW())
                    ON CONFLICT (quarter) 
                    DO UPDATE SET 
                        eps_estimated = EXCLUDED.eps_estimated,
                        updated_at = NOW()
                    "#,
                    quarter,
                    eps_decimal,
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_historical_year(&self, year: i32) -> Result<Option<HistoricalRecord>, Box<dyn Error>> {
        let record = sqlx::query!(
            "SELECT year, sp500_price, dividend, eps, cape FROM historical_data WHERE year = $1",
            year
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| HistoricalRecord {
            year: r.year,
            sp500_price: r.sp500_price.to_f64().unwrap_or(0.0),
            dividend: r.dividend.to_f64().unwrap_or(0.0),
            eps: r.eps.to_f64().unwrap_or(0.0),
            cape: r.cape.to_f64().unwrap_or(0.0),
        }))
    }

    pub async fn update_historical_record(&self, record: HistoricalRecord) -> Result<(), Box<dyn Error>> {
        sqlx::query!(
            r#"
            INSERT INTO historical_data (year, sp500_price, dividend, eps, cape)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (year) DO UPDATE SET
                sp500_price = EXCLUDED.sp500_price,
                dividend = EXCLUDED.dividend,
                eps = EXCLUDED.eps,
                cape = EXCLUDED.cape,
                last_updated = NOW()
            "#,
            record.year,
            BigDecimal::from_f64(record.sp500_price).ok_or("Failed to convert sp500_price")?,
            BigDecimal::from_f64(record.dividend).ok_or("Failed to convert dividend")?,
            BigDecimal::from_f64(record.eps).ok_or("Failed to convert eps")?,
            BigDecimal::from_f64(record.cape).ok_or("Failed to convert cape")?,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
    
    pub async fn get_historical_data(&self) -> Result<Vec<HistoricalRecord>, Box<dyn Error>> {
        let records = sqlx::query!(
            "SELECT year, sp500_price, dividend, eps, cape FROM historical_data ORDER BY year"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records.into_iter().map(|r| HistoricalRecord {
            year: r.year,
            sp500_price: r.sp500_price.to_f64().unwrap_or(0.0),
            dividend: r.dividend.to_f64().unwrap_or(0.0),
            eps: r.eps.to_f64().unwrap_or(0.0),
            cape: r.cape.to_f64().unwrap_or(0.0),
        }).collect())
    }
}