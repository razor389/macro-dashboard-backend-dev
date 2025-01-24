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
    
    //println!("Opening CSV file...");
    // Read CSV file
    let file = File::open("stk_mkt.csv")?;
    //println!("File opened successfully");
    let mut rdr = Reader::from_reader(file);
    //println!("CSV reader created");
    // Insert data from CSV
    for result in rdr.records() {
        //println!("Processing record...");
        let record = result?;
        //println!("Record content: {:?}", record);

        if &record[0] == "Year" {
            //println!("SKipping header row");
            continue;
        }
        
        let year: i32 = record[0].trim().parse()?;
        let sp500_price = BigDecimal::from_str(record[1].trim())?;
        let dividend = BigDecimal::from_str(record[2].trim())?;
        let eps = match record[3].trim(){
            "" => BigDecimal::from(0),
            val => BigDecimal::from_str(val)?
        };
        let cape = BigDecimal::from_str(record[4].trim())?;
        //println!("Parsed year: {}, price: {}, dividend: {}, eps: {}, cape: {}", year, sp500_price, dividend, eps, cape);
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
