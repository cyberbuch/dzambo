CREATE TABLE IF NOT EXISTS bank_matching_rules (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    keywords TEXT NOT NULL DEFAULT '',
    direction TEXT NOT NULL DEFAULT 'any',
    priority INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    account_id BIGINT NOT NULL DEFAULT 0,
    counterpart_id BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS bank_rules_company_idx ON bank_matching_rules (company_id);
