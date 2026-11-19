-- Фаза 1: потребители, фирми, членства, сметкоплан, фактури (за таблото), банки.
-- Diesel работи с таблици до 32 колони, затова дългата фирмена опашка
-- (счетоводител, софтуер, адресна разбивка) е в company_details.

CREATE TABLE users (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    is_super_admin BOOLEAN NOT NULL DEFAULT false,
    role_id INTEGER NOT NULL DEFAULT 0,
    totp_enabled BOOLEAN NOT NULL DEFAULT false,
    session_ver INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Глобални настройки + активна фирма (ключ active_company:{sub}).
CREATE TABLE settings (
    setting_key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE companies (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name TEXT NOT NULL,
    eik TEXT NOT NULL DEFAULT '',
    vat_number TEXT NOT NULL DEFAULT '',
    is_vat_registered BOOLEAN NOT NULL DEFAULT false,
    vat_period TEXT NOT NULL DEFAULT 'monthly',
    currency TEXT NOT NULL DEFAULT 'EUR',
    address TEXT NOT NULL DEFAULT '',
    city TEXT NOT NULL DEFAULT '',
    post_code TEXT NOT NULL DEFAULT '',
    phone TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT '',
    website TEXT NOT NULL DEFAULT '',
    mol TEXT NOT NULL DEFAULT '',
    manager_eik TEXT NOT NULL DEFAULT '',
    accountant_name TEXT NOT NULL DEFAULT '',
    nap_office TEXT NOT NULL DEFAULT '',
    iban TEXT NOT NULL DEFAULT '',
    bic TEXT NOT NULL DEFAULT '',
    inventory_valuation_method TEXT NOT NULL DEFAULT 'WAC',
    fiscal_year_start_month INTEGER NOT NULL DEFAULT 1,
    country TEXT NOT NULL DEFAULT 'BG',
    manager_name TEXT NOT NULL DEFAULT '',
    region TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 1:1 с companies; company_id е и първичният ключ.
CREATE TABLE company_details (
    company_id BIGINT PRIMARY KEY REFERENCES companies(id) ON DELETE CASCADE,
    accountant_egn TEXT NOT NULL DEFAULT '',
    manager_egn TEXT NOT NULL DEFAULT '',
    tax_authority TEXT NOT NULL DEFAULT '',
    authorized_person_name TEXT NOT NULL DEFAULT '',
    authorized_person_egn TEXT NOT NULL DEFAULT '',
    vat_branch_number TEXT NOT NULL DEFAULT '',
    tax_accounting_basis TEXT NOT NULL DEFAULT '',
    tax_entity TEXT NOT NULL DEFAULT '',
    software_company_name TEXT NOT NULL DEFAULT '',
    software_id TEXT NOT NULL DEFAULT '',
    software_version TEXT NOT NULL DEFAULT '',
    is_part_of_group TEXT NOT NULL DEFAULT '',
    street_name TEXT NOT NULL DEFAULT '',
    building_number TEXT NOT NULL DEFAULT '',
    address_building TEXT NOT NULL DEFAULT '',
    additional_address_detail TEXT NOT NULL DEFAULT '',
    address_type TEXT NOT NULL DEFAULT '',
    settings JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE user_companies (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    role_id INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, company_id)
);

CREATE TABLE accounts (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    number TEXT NOT NULL,
    name TEXT NOT NULL,
    saft_account_type TEXT NOT NULL DEFAULT '',
    analytic_type TEXT NOT NULL DEFAULT 'none',
    UNIQUE (company_id, number)
);

-- Фаза 1: минимални колони за таблото; фаза 3 разширява за фактури.
CREATE TABLE invoices (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    number TEXT NOT NULL,
    issue_date DATE NOT NULL,
    counterpart_id BIGINT NOT NULL DEFAULT 0,
    counterpart_name TEXT NOT NULL DEFAULT '',
    currency TEXT NOT NULL DEFAULT 'EUR',
    net_amount NUMERIC(18, 4) NOT NULL DEFAULT 0,
    vat_amount NUMERIC(18, 4) NOT NULL DEFAULT 0,
    total_amount NUMERIC(18, 4) NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'draft',
    direction TEXT NOT NULL DEFAULT 'out',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX invoices_company_idx ON invoices (company_id);

CREATE TABLE bank_accounts (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    balance NUMERIC(18, 4) NOT NULL DEFAULT 0
);
CREATE INDEX bank_accounts_company_idx ON bank_accounts (company_id);
