CREATE TABLE saft_account_mappings (
    id BIGSERIAL PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    type_code TEXT NOT NULL,
    debit_account TEXT NOT NULL,
    credit_account TEXT NOT NULL,
    debit_analytical TEXT NOT NULL DEFAULT '',
    credit_analytical TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);
CREATE INDEX idx_smap_company ON saft_account_mappings (company_id, kind);
