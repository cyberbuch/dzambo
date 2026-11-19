CREATE TABLE inventory_lots (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    product_id BIGINT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    invoice_id BIGINT NOT NULL DEFAULT 0,
    invoice_line_id BIGINT NOT NULL DEFAULT 0,
    received_date DATE NOT NULL,
    quantity NUMERIC(18,4) NOT NULL DEFAULT 0,
    remaining NUMERIC(18,4) NOT NULL DEFAULT 0,
    unit_cost NUMERIC(18,4) NOT NULL DEFAULT 0,
    location_id BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX inventory_lots_company_idx ON inventory_lots (company_id);
CREATE INDEX inventory_lots_product_idx ON inventory_lots (product_id);
