// src/services/sheets.rs

use serde::{Deserialize, Serialize};
use std::error::Error;
use crate::{models::QuarterlyData, services::google_oauth::fetch_access_token_from_file};
use log::info;
use serde_json::json;
use reqwest::Client;
use crate::models::HistoricalRecord;

#[derive(Clone)]
pub struct SheetsConfig {
    pub spreadsheet_id: String,
    // Instead of `api_key`, store the path to your service account JSON
    pub service_account_json_path: String,
}

// Represents the structure of our sheets
pub struct SheetNames {
    pub market_cache: &'static str,
    pub quarterly_data: &'static str,
    pub historical_data: &'static str,
}

impl Default for SheetNames {
    fn default() -> Self {
        SheetNames {
            market_cache: "MarketCache",
            quarterly_data: "QuarterlyData",
            historical_data: "HistoricalData",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawMarketCache {
    pub timestamp_yahoo: String,
    pub timestamp_ycharts: String,
    pub timestamp_treasury: String,  // New: timestamp for treasury data
    pub timestamp_bls: String,      // New: timestamp for BLS data
    pub daily_close_sp500_price: f64,
    pub current_sp500_price: f64,
    pub current_cape: f64,
    pub cape_period: String,
    pub tips_yield_20y: f64,        // New: 20yr TIPS yield
    pub bond_yield_20y: f64,        // New: 20yr bond yield
    pub tbill_yield: f64,          // New: T-bill yield
    pub inflation_rate: f64,        // New: inflation rate
}

pub struct SheetsStore {
    pub config: SheetsConfig,
    client: Client,
    sheet_names: SheetNames,
}

impl SheetsStore {
    pub fn new(config: SheetsConfig) -> Self {
        SheetsStore {
            config,
            client: reqwest::Client::new(),
            sheet_names: SheetNames::default(),
        }
    }

    pub async fn get_auth_token(&self) -> Result<String, Box<dyn Error>> {
        crate::services::google_oauth::fetch_access_token_from_file(&self.config.service_account_json_path).await
    }

    pub async fn bulk_upload_historical_records(&self, records: &[HistoricalRecord]) -> Result<(), Box<dyn Error>> {
        let token = self.get_auth_token().await?;
        let client = reqwest::Client::new();
        
        // Convert records to values, using empty string for zero values
        let values: Vec<Vec<String>> = records.iter()
            .map(|record| vec![
                record.year.to_string(),
                if record.sp500_price == 0.0 { "".to_string() } else { record.sp500_price.to_string() },
                if record.dividend == 0.0 { "".to_string() } else { record.dividend.to_string() },
                if record.dividend_yield == 0.0 { "".to_string() } else { record.dividend_yield.to_string() },
                if record.eps == 0.0 { "".to_string() } else { record.eps.to_string() },
                if record.cape == 0.0 { "".to_string() } else { record.cape.to_string() },
                if record.inflation == 0.0 { "".to_string() } else { record.inflation.to_string() },
                if record.total_return == 0.0 { "".to_string() } else { record.total_return.to_string() },
                if record.cumulative_return == 0.0 { "".to_string() } else { record.cumulative_return.to_string() },
            ])
            .collect();
    
        let range = format!("{}!A2:I{}", self.sheet_names.historical_data, values.len() + 1);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            self.config.spreadsheet_id,
            range
        );
    
        let body = json!({
            "values": values,
            "majorDimension": "ROWS"
        });
    
        let response = client
            .put(&url)
            .header("Content-Type", "application/json")
            .query(&[("valueInputOption", "RAW")])
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?;
    
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Failed to upload historical records: {}", error_text).into());
        }
    
        Ok(())
    }    

    /// Example of reading from the "MarketCache!A2:F2" range
    pub async fn get_market_cache(&self) -> Result<RawMarketCache, Box<dyn Error>> {
        let token = fetch_access_token_from_file(&self.config.service_account_json_path).await?;

        let range = format!("{}!A2:L2", self.sheet_names.market_cache);  // Updated range
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            self.config.spreadsheet_id, range
        );

        let response: serde_json::Value = self.client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if let Some(values) = response["values"].as_array() {
            if let Some(row) = values.first() {
                return Ok(RawMarketCache {
                    timestamp_yahoo: row.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    timestamp_ycharts: row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    timestamp_treasury: row.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    timestamp_bls: row.get(3).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    daily_close_sp500_price: row.get(4).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    current_sp500_price: row.get(5).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    current_cape: row.get(6).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    cape_period: row.get(7).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    tips_yield_20y: row.get(8).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    bond_yield_20y: row.get(9).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    tbill_yield: row.get(10).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    inflation_rate: row.get(11).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                });
            }
        }

        Err("No market cache data found".into())
    }

    pub async fn update_market_cache(&self, cache: &RawMarketCache) -> Result<(), Box<dyn Error>> {
        let token = fetch_access_token_from_file(&self.config.service_account_json_path).await?;

        let range = format!("{}!A2:L2", self.sheet_names.market_cache);  // Updated range
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=RAW",
            self.config.spreadsheet_id, range
        );

        let values = vec![vec![
            cache.timestamp_yahoo.to_string(),
            cache.timestamp_ycharts.to_string(),
            cache.timestamp_treasury.to_string(),
            cache.timestamp_bls.to_string(),
            cache.daily_close_sp500_price.to_string(),
            cache.current_sp500_price.to_string(),
            cache.current_cape.to_string(),
            cache.cape_period.clone(),
            cache.tips_yield_20y.to_string(),
            cache.bond_yield_20y.to_string(),
            cache.tbill_yield.to_string(),
            cache.inflation_rate.to_string(),
        ]];

        let body = json!({
            "values": values,
        });

        let resp = self.client
            .put(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        info!("update_market_cache response: {:?}", resp);
        Ok(())
    }

    /// Example of reading from "QuarterlyData!A2:D" range
    pub async fn get_quarterly_data(&self) -> Result<Vec<QuarterlyData>, Box<dyn Error>> {
        let token = fetch_access_token_from_file(&self.config.service_account_json_path).await?;

        let range = format!("{}!A2:D", self.sheet_names.quarterly_data);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            self.config.spreadsheet_id, range
        );

        let response: serde_json::Value = self.client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let mut quarterly_data = Vec::new();
        if let Some(values) = response["values"].as_array() {
            for row in values {
                let quarter = row.get(0).and_then(|v| v.as_str()).unwrap_or("");
                let dividend = row.get(1).and_then(|v| v.as_str()).unwrap_or("").parse().ok();
                let eps_actual = row.get(2).and_then(|v| v.as_str()).unwrap_or("").parse().ok();
                let eps_estimated = row.get(3).and_then(|v| v.as_str()).unwrap_or("").parse().ok();

                quarterly_data.push(QuarterlyData {
                    quarter: quarter.to_string(),
                    dividend,
                    eps_actual,
                    eps_estimated,
                });
            }
        }
        Ok(quarterly_data)
    }

    pub async fn update_quarterly_data(&self, data: &[QuarterlyData]) -> Result<(), Box<dyn Error>> {
        let token = fetch_access_token_from_file(&self.config.service_account_json_path).await?;

        let range = format!("{}!A2:D{}", self.sheet_names.quarterly_data, data.len() + 1);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=RAW",
            self.config.spreadsheet_id, range
        );

        let values: Vec<Vec<String>> = data.iter().map(|row| {
            vec![
                row.quarter.clone(),
                row.dividend.map(|v| v.to_string()).unwrap_or_default(),
                row.eps_actual.map(|v| v.to_string()).unwrap_or_default(),
                row.eps_estimated.map(|v| v.to_string()).unwrap_or_default(),
            ]
        }).collect();

        let body = json!({
            "values": values,
        });

        let resp = self.client
            .put(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        info!("update_quarterly_data response: {:?}", resp);
        Ok(())
    }

    pub async fn get_historical_data(&self) -> Result<Vec<HistoricalRecord>, Box<dyn Error>> {
        let token = fetch_access_token_from_file(&self.config.service_account_json_path).await?;
    
        let range = format!("{}!A2:I", self.sheet_names.historical_data);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
            self.config.spreadsheet_id, range
        );
    
        let response: serde_json::Value = self.client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
    
        let mut historical_data = Vec::new();
        if let Some(values) = response["values"].as_array() {
            for row in values {
                // Helper function to parse optional float value
                let parse_opt_float = |value: Option<&serde_json::Value>| -> f64 {
                    value
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.0)
                };
    
                historical_data.push(HistoricalRecord {
                    year: row.get(0).and_then(|v| v.as_str()).unwrap_or("0").parse()?,
                    sp500_price: parse_opt_float(row.get(1)),
                    dividend: parse_opt_float(row.get(2)),
                    dividend_yield: parse_opt_float(row.get(3)),
                    eps: parse_opt_float(row.get(4)),
                    cape: parse_opt_float(row.get(5)),
                    inflation: parse_opt_float(row.get(6)),
                    total_return: parse_opt_float(row.get(7)),
                    cumulative_return: parse_opt_float(row.get(8)),
                });
            }
        }
    
        Ok(historical_data)
    }

    pub async fn update_historical_record(&self, record: &HistoricalRecord) -> Result<(), Box<dyn Error>> {
        let all_records = self.get_historical_data().await?;
        let row_index = all_records.iter().position(|r| r.year == record.year)
            .ok_or("Record not found")?;
    
        let token = fetch_access_token_from_file(&self.config.service_account_json_path).await?;
    
        let row_num = row_index + 2;
        let range = format!("{}!A{}:I{}", self.sheet_names.historical_data, row_num, row_num);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?valueInputOption=RAW",
            self.config.spreadsheet_id, range
        );
    
        let values = vec![vec![
            record.year.to_string(),
            if record.sp500_price == 0.0 { "".to_string() } else { record.sp500_price.to_string() },
            if record.dividend == 0.0 { "".to_string() } else { record.dividend.to_string() },
            if record.dividend_yield == 0.0 { "".to_string() } else { record.dividend_yield.to_string() },
            if record.eps == 0.0 { "".to_string() } else { record.eps.to_string() },
            if record.cape == 0.0 { "".to_string() } else { record.cape.to_string() },
            if record.inflation == 0.0 { "".to_string() } else { record.inflation.to_string() },
            if record.total_return == 0.0 { "".to_string() } else { record.total_return.to_string() },
            if record.cumulative_return == 0.0 { "".to_string() } else { record.cumulative_return.to_string() },
        ]];
    
        let body = json!({
            "values": values,
        });
    
        let response = self.client
            .put(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
    
        info!("update_historical_record response: {:?}", response);
        Ok(())
    }
}
