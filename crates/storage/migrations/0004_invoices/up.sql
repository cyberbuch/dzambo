-- Фаза 3: фактури (пълна схема вместо фазата-1 огледало), дневник, периоди.
DROP TABLE IF EXISTS invoice_lines;
DROP TABLE IF EXISTS invoices;

CREATE TABLE invoices (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    direction TEXT NOT NULL DEFAULT 'out',
    document_type TEXT NOT NULL DEFAULT '01',
    number TEXT NOT NULL,
    issue_date DATE NOT NULL,
    due_date DATE,
    counterpart_id BIGINT NOT NULL DEFAULT 0,
    counterpart_name TEXT NOT NULL DEFAULT '',
    currency TEXT NOT NULL DEFAULT 'EUR',
    currency_rate NUMERIC(18, 6) NOT NULL DEFAULT 1,
    net_amount NUMERIC(18, 4) NOT NULL DEFAULT 0,
    vat_amount NUMERIC(18, 4) NOT NULL DEFAULT 0,
    total_amount NUMERIC(18, 4) NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'draft',
    payment_method TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    accounting_month TEXT NOT NULL DEFAULT '',
    original_invoice_id BIGINT NOT NULL DEFAULT 0,
    journal_entry_id BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX invoices_company_idx ON invoices (company_id);

CREATE TABLE invoice_lines (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    invoice_id BIGINT NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    quantity NUMERIC(18, 4) NOT NULL DEFAULT 1,
    unit TEXT NOT NULL DEFAULT '',
    unit_price NUMERIC(18, 4) NOT NULL DEFAULT 0,
    vat_rate NUMERIC(18, 4) NOT NULL DEFAULT 20,
    product_id BIGINT NOT NULL DEFAULT 0,
    net_amount NUMERIC(18, 4) NOT NULL DEFAULT 0,
    vat_amount NUMERIC(18, 4) NOT NULL DEFAULT 0,
    total_amount NUMERIC(18, 4) NOT NULL DEFAULT 0
);
CREATE INDEX invoice_lines_invoice_idx ON invoice_lines (invoice_id);

CREATE TABLE journal_entries (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    entry_date DATE NOT NULL,
    document_type TEXT NOT NULL DEFAULT 'manual',
    document_id BIGINT NOT NULL DEFAULT 0,
    document_number TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    vat_type TEXT NOT NULL DEFAULT 'no_vat',
    counterpart_id BIGINT NOT NULL DEFAULT 0,
    counterpart_name TEXT NOT NULL DEFAULT '',
    document_date DATE NOT NULL,
    vat_period TEXT NOT NULL DEFAULT ''
);
CREATE INDEX journal_entries_company_idx ON journal_entries (company_id);

CREATE TABLE journal_lines (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    entry_id BIGINT NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    account_id BIGINT NOT NULL,
    account_number TEXT NOT NULL DEFAULT '',
    account_name TEXT NOT NULL DEFAULT '',
    direction TEXT NOT NULL,
    amount NUMERIC(18, 4) NOT NULL DEFAULT 0,
    vat_amount NUMERIC(18, 4) NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT '',
    fx_quantity NUMERIC(18, 4) NOT NULL DEFAULT 0,
    fx_rate NUMERIC(18, 6) NOT NULL DEFAULT 1
);
CREATE INDEX journal_lines_entry_idx ON journal_lines (entry_id);

CREATE TABLE accounting_periods (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    year INTEGER NOT NULL,
    month INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    closed_at TIMESTAMPTZ,
    notes TEXT NOT NULL DEFAULT '',
    UNIQUE (company_id, year, month)
);
