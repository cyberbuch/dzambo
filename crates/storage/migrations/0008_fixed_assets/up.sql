CREATE TABLE fixed_asset_categories (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    cita_category TEXT NOT NULL DEFAULT '',
    min_depreciation_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    max_depreciation_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    default_method TEXT NOT NULL DEFAULT 'straight_line',
    asset_account_id BIGINT NOT NULL DEFAULT 0,
    depreciation_account_id BIGINT NOT NULL DEFAULT 0,
    expense_account_id BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE fixed_assets (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    inventory_number TEXT NOT NULL DEFAULT '',
    category_id BIGINT NOT NULL DEFAULT 0,
    acquisition_date DATE NOT NULL,
    put_into_service_date DATE,
    cost NUMERIC(18,4) NOT NULL DEFAULT 0,
    salvage_value NUMERIC(18,4) NOT NULL DEFAULT 0,
    useful_life_months INTEGER NOT NULL DEFAULT 0,
    depreciation_method TEXT NOT NULL DEFAULT 'straight_line',
    accounting_depreciation_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    tax_depreciation_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    accumulated_depreciation NUMERIC(18,4) NOT NULL DEFAULT 0,
    tax_accumulated_depreciation NUMERIC(18,4) NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active',
    serial_number TEXT NOT NULL DEFAULT '',
    manufacturer TEXT NOT NULL DEFAULT '',
    model TEXT NOT NULL DEFAULT '',
    responsible_person TEXT NOT NULL DEFAULT '',
    is_conserved BOOLEAN NOT NULL DEFAULT false,
    location_id BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX fixed_assets_company_idx ON fixed_assets (company_id);
