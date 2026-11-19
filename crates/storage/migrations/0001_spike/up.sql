-- Сонда на типовете: NUMERIC(18,4) за пари, TIMESTAMPTZ за време.
CREATE TABLE spike_entries (
    id SERIAL PRIMARY KEY,
    amount NUMERIC(18, 4) NOT NULL,
    note TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX spike_entries_amount_idx ON spike_entries (amount);
