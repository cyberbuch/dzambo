CREATE TABLE stock_documents (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    number TEXT NOT NULL,
    doc_date DATE NOT NULL,
    kind TEXT NOT NULL DEFAULT 'issue',
    from_location_id BIGINT NOT NULL DEFAULT 0,
    to_location_id BIGINT NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'posted',
    notes TEXT NOT NULL DEFAULT ''
);

CREATE TABLE stock_document_lines (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    document_id BIGINT NOT NULL REFERENCES stock_documents(id) ON DELETE CASCADE,
    product_id BIGINT NOT NULL,
    quantity NUMERIC(18,4) NOT NULL DEFAULT 0,
    unit_cost NUMERIC(18,4) NOT NULL DEFAULT 0,
    description TEXT NOT NULL DEFAULT ''
);
CREATE INDEX stock_doc_lines_idx ON stock_document_lines (document_id);
