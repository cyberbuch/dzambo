CREATE TABLE exchange_rates (
    id BIGSERIAL PRIMARY KEY,
    currency_code TEXT NOT NULL,
    rate NUMERIC(18,6) NOT NULL,
    rate_date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (currency_code, rate_date)
);
CREATE INDEX idx_rates_date ON exchange_rates (rate_date DESC);

-- BGN е фиксиран към EUR (1.95583) от 1999 г. — курсът важи за всяка дата.
INSERT INTO exchange_rates (currency_code, rate, rate_date)
VALUES ('BGN', 1.95583, DATE '1999-01-01')
ON CONFLICT (currency_code, rate_date) DO NOTHING;