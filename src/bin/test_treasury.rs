// src/bin/test_treasury.rs
use macro_dashboard_acm::services::treasury::{fetch_tbill_data};
use macro_dashboard_acm::services::treasury_long::{fetch_20y_bond_yield, fetch_20y_tips_yield};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>{
    println!("20y Nominal Yield:   {:?}", fetch_20y_bond_yield().await?);
    println!("20y TIPS Yield:      {:?}", fetch_20y_tips_yield().await?);
    println!("4-Week T-Bill Yield: {:?}", fetch_tbill_data().await?);
    Ok(())
}
