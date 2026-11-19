-- Фаза 2: номенклатури — контрагенти, стоки, настройки на фирмата,
-- и is_currency по сметките от групите 4xx/5xx.

ALTER TABLE accounts ADD COLUMN is_currency BOOLEAN NOT NULL DEFAULT false;
UPDATE accounts SET is_currency = true
WHERE number LIKE '4%' OR number LIKE '5%';

CREATE TABLE counterparts (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    counterpart_type TEXT NOT NULL DEFAULT 'both',
    eik TEXT NOT NULL DEFAULT '',
    vat_number TEXT NOT NULL DEFAULT '',
    address TEXT NOT NULL DEFAULT '',
    city TEXT NOT NULL DEFAULT '',
    post_code TEXT NOT NULL DEFAULT '',
    country TEXT NOT NULL DEFAULT 'BG',
    contact_person TEXT NOT NULL DEFAULT ''
);
CREATE INDEX counterparts_company_idx ON counterparts (company_id);

-- 1:1 с counterparts; id е и външният ключ.
CREATE TABLE counterpart_extra (
    id BIGINT PRIMARY KEY REFERENCES counterparts(id) ON DELETE CASCADE,
    vies_address TEXT NOT NULL DEFAULT '',
    street_name TEXT NOT NULL DEFAULT '',
    building_number TEXT NOT NULL DEFAULT '',
    address_building TEXT NOT NULL DEFAULT '',
    additional_address_detail TEXT NOT NULL DEFAULT '',
    address_type TEXT NOT NULL DEFAULT '',
    region TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT '',
    phone TEXT NOT NULL DEFAULT ''
);

CREATE TABLE products (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    unit TEXT NOT NULL DEFAULT 'C62',
    price NUMERIC(18, 4) NOT NULL DEFAULT 0,
    cost_price NUMERIC(18, 4) NOT NULL DEFAULT 0,
    vat_rate NUMERIC(18, 4) NOT NULL DEFAULT 20,
    is_service BOOLEAN NOT NULL DEFAULT false,
    is_inventory BOOLEAN NOT NULL DEFAULT false,
    inventory_account_id BIGINT NOT NULL DEFAULT 0,
    revenue_account_id BIGINT NOT NULL DEFAULT 0,
    stock_quantity NUMERIC(18, 4) NOT NULL DEFAULT 0,
    stock_value NUMERIC(18, 4) NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true
);
CREATE INDEX products_company_idx ON products (company_id);

CREATE TABLE product_extra (
    id BIGINT PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
    product_type TEXT NOT NULL DEFAULT '',
    cn8_code TEXT NOT NULL DEFAULT '',
    cn8_name TEXT NOT NULL DEFAULT '',
    expense_account_id BIGINT NOT NULL DEFAULT 0,
    auto_write_off BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE company_locations (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    location_type TEXT NOT NULL DEFAULT 'OFFICE',
    street_name TEXT NOT NULL DEFAULT '',
    building_number TEXT NOT NULL DEFAULT '',
    city TEXT NOT NULL DEFAULT '',
    post_code TEXT NOT NULL DEFAULT '',
    region TEXT NOT NULL DEFAULT '',
    country TEXT NOT NULL DEFAULT 'BG',
    is_main BOOLEAN NOT NULL DEFAULT false
);
CREATE INDEX company_locations_company_idx ON company_locations (company_id);

CREATE TABLE beneficial_owners (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    first_name_bg TEXT NOT NULL DEFAULT '',
    last_name_bg TEXT NOT NULL DEFAULT '',
    egn TEXT NOT NULL DEFAULT '',
    first_name_latin TEXT NOT NULL DEFAULT '',
    last_name_latin TEXT NOT NULL DEFAULT '',
    country TEXT NOT NULL DEFAULT 'BG',
    ownership_percentage TEXT NOT NULL DEFAULT ''
);
CREATE INDEX beneficial_owners_company_idx ON beneficial_owners (company_id);

CREATE TABLE ultimate_parents (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name_bg TEXT NOT NULL,
    uic TEXT NOT NULL DEFAULT '',
    name_latin TEXT NOT NULL DEFAULT '',
    country TEXT NOT NULL DEFAULT 'BG'
);
CREATE INDEX ultimate_parents_company_idx ON ultimate_parents (company_id);

CREATE TABLE document_series (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    document_types TEXT NOT NULL DEFAULT '',
    pad_width INTEGER NOT NULL DEFAULT 8,
    start_number INTEGER NOT NULL DEFAULT 1,
    next_number TEXT NOT NULL DEFAULT '1'
);
CREATE INDEX document_series_company_idx ON document_series (company_id);
