// src/models.rs
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Timestamps {
    pub yahoo_price: DateTime<Utc>,
    pub ycharts_data: DateTime<Utc>,
    pub treasury_data: DateTime<Utc>,  // New
    pub bls_data: DateTime<Utc>,       // New
}


#[derive(Debug, Clone)]
pub struct MarketCache {
    pub timestamps: Timestamps,
    pub daily_close_sp500_price: f64,
    pub current_sp500_price: f64,
    pub quarterly_dividends: HashMap<String, f64>,
    pub eps_actual: HashMap<String, f64>,
    pub eps_estimated: HashMap<String, f64>,
    pub current_cape: f64,
    pub cape_period: String,
    pub tips_yield_20y: f64,        // New
    pub bond_yield_20y: f64,        // New
    pub tbill_yield: f64,          // New
    pub inflation_rate: f64,        // New
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalRecord {
    pub year: i32,
    pub sp500_price: f64,
    pub dividend: f64,
    pub dividend_yield: f64,
    pub eps: f64,
    pub cape: f64,
    pub inflation: f64,
    pub total_return: f64,
    pub cumulative_return: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuarterlyData {
    pub quarter: String,
    pub dividend: Option<f64>,
    pub eps_actual: Option<f64>,
    pub eps_estimated: Option<f64>,
}