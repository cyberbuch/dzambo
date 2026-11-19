CREATE TABLE dividend_distributions (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    year INTEGER NOT NULL,
    total_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    decision_date DATE NOT NULL,
    decision_number TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'declared',
    notes TEXT NOT NULL DEFAULT ''
);

CREATE TABLE dividends (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    distribution_id BIGINT NOT NULL REFERENCES dividend_distributions(id) ON DELETE CASCADE,
    beneficial_owner_id BIGINT NOT NULL DEFAULT 0,
    owner_name TEXT NOT NULL DEFAULT '',
    owner_egn TEXT NOT NULL DEFAULT '',
    ownership_percentage TEXT NOT NULL DEFAULT '',
    gross_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    tax_rate NUMERIC(18,4) NOT NULL DEFAULT 5,
    tax_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    net_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    decision_date DATE NOT NULL,
    payment_date DATE,
    is_paid BOOLEAN NOT NULL DEFAULT false,
    notes TEXT NOT NULL DEFAULT ''
);
CREATE INDEX dividends_company_idx ON dividends (company_id);
