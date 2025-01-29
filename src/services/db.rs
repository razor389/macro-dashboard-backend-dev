// src/services/db.rs

use std::error::Error;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::services::sheets::{SheetsStore, SheetsConfig, RawMarketCache};
use crate::models::{MarketCache, Timestamps, HistoricalRecord};

pub struct DbStore {
    sheets_store: SheetsStore,
}

impl DbStore {
    pub async fn new(
        spreadsheet_id: &str,
        service_account_json_path: &str
    ) -> Result<Self, Box<dyn Error>> {
        let config = SheetsConfig {
            spreadsheet_id: spreadsheet_id.to_string(),
            service_account_json_path: service_account_json_path.to_string(),
        };

        let sheets_store = SheetsStore::new(config);

        Ok(DbStore {
            sheets_store
        })
    }

    pub async fn get_market_cache(&self) -> Result<MarketCache, Box<dyn Error>> {
        let raw_cache: RawMarketCache = self.sheets_store.get_market_cache().await?;

        Ok(MarketCache {
            timestamps: Timestamps {
                yahoo_price: DateTime::parse_from_rfc3339(&raw_cache.timestamp_yahoo)?.with_timezone(&Utc),
                ycharts_data: DateTime::parse_from_rfc3339(&raw_cache.timestamp_ycharts)?.with_timezone(&Utc),
            },
            daily_close_sp500_price: raw_cache.daily_close_sp500_price,
            current_sp500_price: raw_cache.current_sp500_price,
            quarterly_dividends: HashMap::new(), // fill these from sheet if you want
            eps_actual: HashMap::new(),          // likewise
            eps_estimated: HashMap::new(),
            current_cape: raw_cache.current_cape,
            cape_period: raw_cache.cape_period,
        })
    }

    pub async fn update_market_cache(&self, cache: &MarketCache) -> Result<(), Box<dyn Error>> {
        // Convert MarketCache -> RawMarketCache
        let raw_cache = RawMarketCache {
            timestamp_yahoo: cache.timestamps.yahoo_price.to_rfc3339(),
            timestamp_ycharts: cache.timestamps.ycharts_data.to_rfc3339(),
            daily_close_sp500_price: cache.daily_close_sp500_price,
            current_sp500_price: cache.current_sp500_price,
            current_cape: cache.current_cape,
            cape_period: cache.cape_period.clone(),
        };

        self.sheets_store.update_market_cache(&raw_cache).await?;
        Ok(())
    }

    pub async fn get_historical_data(&self) -> Result<Vec<HistoricalRecord>, Box<dyn Error>> {
        self.sheets_store.get_historical_data().await
    }

    pub async fn get_historical_year(&self, year: i32) -> Result<Option<HistoricalRecord>, Box<dyn Error>> {
        let records = self.sheets_store.get_historical_data().await?;
        Ok(records.into_iter().find(|r| r.year == year))
    }

    pub async fn update_historical_record(&self, record: HistoricalRecord) -> Result<(), Box<dyn Error>> {
        self.sheets_store.update_historical_record(&record).await
    }
}
