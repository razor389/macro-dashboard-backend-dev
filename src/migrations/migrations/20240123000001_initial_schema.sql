-- migrations/20240123000001_initial_schema.sql
CREATE TABLE market_cache (
    id SERIAL PRIMARY KEY,
    sp500_price DECIMAL NOT NULL,
    current_cape DECIMAL NOT NULL,
    last_yahoo_update TIMESTAMP WITH TIME ZONE NOT NULL,
    last_ycharts_update TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE TABLE quarterly_data (
    quarter VARCHAR(6) PRIMARY KEY, -- Format: 2024Q1
    ttm_dividend DECIMAL,
    eps_actual DECIMAL,
    eps_estimated DECIMAL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE TABLE historical_data (
    year INTEGER PRIMARY KEY,
    sp500_price DECIMAL NOT NULL,
    dividend DECIMAL NOT NULL,
    eps DECIMAL NOT NULL,
    cape DECIMAL NOT NULL
);