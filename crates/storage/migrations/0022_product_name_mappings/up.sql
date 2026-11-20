CREATE TABLE product_name_mappings (
    id BIGSERIAL PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    counterpart_id BIGINT NOT NULL REFERENCES counterparts(id) ON DELETE CASCADE,
    scanned_name TEXT NOT NULL,
    product_id BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (company_id, counterpart_id, scanned_name)
);
