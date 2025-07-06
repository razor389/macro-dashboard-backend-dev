use macro_dashboard_acm::services::equity;
use log::{info, error};
use env_logger;
use dotenv::dotenv;
use chrono::{Utc, NaiveTime, Datelike};
use chrono_tz::US::Central;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();
    
    info!("Testing update timing logic...");
    
    let current_utc = Utc::now();
    let current_ct = current_utc.with_timezone(&Central);
    
    info!("Current time:");
    info!("  UTC: {}", current_utc);
    info!("  Central: {}", current_ct);
    
    // Test daily update timing (3:30 PM Central)
    let target_time = NaiveTime::from_hms_opt(15, 30, 0).unwrap();
    let current_time = current_ct.time();
    let should_update = current_time >= target_time && 
        current_time < target_time + chrono::Duration::minutes(1);
    
    info!("Daily update timing:");
    info!("  Target time: {} Central", target_time);
    info!("  Current time: {} Central", current_time);
    info!("  Should update now: {}", should_update);
    
    // Check if we're within market hours (9:30 AM - 4:00 PM Central)
    let market_open = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
    let market_close = NaiveTime::from_hms_opt(16, 0, 0).unwrap();
    let in_market_hours = current_time >= market_open && current_time <= market_close;
    
    info!("Market hours (9:30 AM - 4:00 PM Central):");
    info!("  Currently in market hours: {}", in_market_hours);
    
    // Test if it's a weekday
    let is_weekday = match current_ct.weekday() {
        chrono::Weekday::Sat | chrono::Weekday::Sun => false,
        _ => true,
    };
    
    info!("Market day:");
    info!("  Is weekday: {}", is_weekday);
    
    // Show next scheduled update times
    let mut next_update = current_ct.clone();
    if current_time >= target_time {
        // If we're past 3:30 PM today, next update is tomorrow
        next_update = next_update + chrono::Duration::days(1);
    }
    let next_update_with_time = next_update.date_naive().and_time(target_time);
    
    info!("Scheduling:");
    info!("  Next daily update: {} Central", next_update_with_time);
    
    // Test Yahoo Finance 15-minute logic
    info!("Yahoo Finance updates every 15 minutes during market hours");
    let last_yahoo_update = current_utc - chrono::Duration::minutes(20); // Simulate last update 20 min ago
    let should_update_yahoo = last_yahoo_update < current_utc - chrono::Duration::minutes(15);
    info!("  Last Yahoo update (simulated): {} UTC", last_yahoo_update);
    info!("  Should update Yahoo now: {}", should_update_yahoo);
    
    Ok(())
}