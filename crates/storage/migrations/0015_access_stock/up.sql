CREATE TABLE roles (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    permissions JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE audit_logs (
    id BIGSERIAL PRIMARY KEY,
    company_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL DEFAULT 0,
    user_login TEXT NOT NULL DEFAULT '',
    user_name TEXT NOT NULL DEFAULT '',
    entity_type TEXT NOT NULL,
    entity_id BIGINT NOT NULL DEFAULT 0,
    entity_number TEXT NOT NULL DEFAULT '',
    action TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    old_amount TEXT NOT NULL DEFAULT '',
    new_amount TEXT NOT NULL DEFAULT '',
    old_data JSONB,
    new_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_audit_company ON audit_logs (company_id, id DESC);

CREATE TABLE locations (
    id BIGSERIAL PRIMARY KEY,
    company_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE internal_docs (
    id BIGSERIAL PRIMARY KEY,
    company_id BIGINT NOT NULL,
    number TEXT NOT NULL,
    doc_date DATE NOT NULL,
    from_location_id BIGINT NOT NULL DEFAULT 0,
    to_location_id BIGINT NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'draft',
    notes TEXT NOT NULL DEFAULT '',
    handed_by TEXT NOT NULL DEFAULT '',
    received_by TEXT NOT NULL DEFAULT ''
);

CREATE TABLE internal_doc_lines (
    id BIGSERIAL PRIMARY KEY,
    doc_id BIGINT NOT NULL,
    line_kind TEXT NOT NULL DEFAULT 'product',
    product_id BIGINT NOT NULL DEFAULT 0,
    asset_id BIGINT NOT NULL DEFAULT 0,
    quantity NUMERIC(18,4) NOT NULL DEFAULT 0,
    unit_cost NUMERIC(18,6) NOT NULL DEFAULT 0,
    amount NUMERIC(18,2) NOT NULL DEFAULT 0,
    description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE material_issues (
    id BIGSERIAL PRIMARY KEY,
    company_id BIGINT NOT NULL,
    number TEXT NOT NULL,
    doc_date DATE NOT NULL,
    location_id BIGINT NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'draft',
    notes TEXT NOT NULL DEFAULT '',
    journal_entry_id BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE material_issue_lines (
    id BIGSERIAL PRIMARY KEY,
    doc_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    quantity NUMERIC(18,4) NOT NULL DEFAULT 0,
    unit_cost NUMERIC(18,6) NOT NULL DEFAULT 0,
    amount NUMERIC(18,2) NOT NULL DEFAULT 0,
    account_id BIGINT NOT NULL DEFAULT 0,
    inventory_account_id BIGINT NOT NULL DEFAULT 0,
    line_no INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE stock_account_rules (
    id BIGSERIAL PRIMARY KEY,
    company_id BIGINT NOT NULL,
    kind TEXT NOT NULL,
    inventory_account_id BIGINT NOT NULL,
    offset_account_id BIGINT NOT NULL,
    clearing_account_id BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE fixed_asset_events (
    id BIGSERIAL PRIMARY KEY,
    company_id BIGINT NOT NULL,
    asset_id BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    event_date DATE NOT NULL,
    amount NUMERIC(18,2) NOT NULL DEFAULT 0,
    location TEXT NOT NULL DEFAULT '',
    reason TEXT NOT NULL DEFAULT ''
);