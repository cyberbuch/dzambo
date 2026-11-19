CREATE TABLE recipes (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    code TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL,
    output_product_id BIGINT NOT NULL DEFAULT 0,
    output_quantity NUMERIC(18,4) NOT NULL DEFAULT 1,
    output_unit TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE recipe_lines (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    recipe_id BIGINT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    product_id BIGINT NOT NULL,
    quantity NUMERIC(18,4) NOT NULL DEFAULT 0,
    unit TEXT NOT NULL DEFAULT '',
    wastage_percent NUMERIC(18,4) NOT NULL DEFAULT 0,
    line_no INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE production_orders (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    number TEXT NOT NULL DEFAULT '',
    recipe_id BIGINT NOT NULL DEFAULT 0,
    output_product_id BIGINT NOT NULL DEFAULT 0,
    quantity NUMERIC(18,4) NOT NULL DEFAULT 0,
    location_id BIGINT NOT NULL DEFAULT 0,
    order_date DATE NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    notes TEXT NOT NULL DEFAULT '',
    material_cost NUMERIC(18,4) NOT NULL DEFAULT 0,
    journal_entry_id BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX production_orders_company_idx ON production_orders (company_id);

CREATE TABLE production_order_lines (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    order_id BIGINT NOT NULL REFERENCES production_orders(id) ON DELETE CASCADE,
    product_id BIGINT NOT NULL,
    product_name TEXT NOT NULL DEFAULT '',
    quantity NUMERIC(18,4) NOT NULL DEFAULT 0,
    unit TEXT NOT NULL DEFAULT '',
    unit_cost NUMERIC(18,4) NOT NULL DEFAULT 0,
    amount NUMERIC(18,4) NOT NULL DEFAULT 0
);
