-- migrations/20240123000001_initial_schema.sql
BEGIN;

CREATE TABLE market_cache (
    id SERIAL PRIMARY KEY,
    daily_close_sp500_price DECIMAL NOT NULL,
    current_sp500_price DECIMAL NOT NULL,
    current_cape DECIMAL NOT NULL,
    cape_period VARCHAR(20) NOT NULL, -- e.g., "Jan 2025"
    last_yahoo_update TIMESTAMP WITH TIME ZONE NOT NULL,
    last_ycharts_update TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE TABLE quarterly_data (
    quarter VARCHAR(6) PRIMARY KEY, -- Format: 2024Q1
    dividend DECIMAL,
    eps_actual DECIMAL,
    eps_estimated DECIMAL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE TABLE historical_data (
    year INTEGER PRIMARY KEY,
    sp500_price DECIMAL NOT NULL,
    dividend DECIMAL NOT NULL,
    eps DECIMAL NOT NULL,
    cape DECIMAL NOT NULL,
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

COMMIT;