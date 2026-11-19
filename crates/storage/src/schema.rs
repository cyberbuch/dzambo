//! Diesel схема. Фаза 0 сондата (spike_entries) си остава за теста.

diesel::table! {
    spike_entries (id) {
        id -> Int4,
        amount -> Numeric,
        note -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Int8,
        email -> Text,
        name -> Text,
        password_hash -> Text,
        is_active -> Bool,
        is_super_admin -> Bool,
        role_id -> Int4,
        totp_enabled -> Bool,
        session_ver -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    settings (setting_key) {
        setting_key -> Text,
        value -> Text,
    }
}

diesel::table! {
    companies (id) {
        id -> Int8,
        name -> Text,
        eik -> Text,
        vat_number -> Text,
        is_vat_registered -> Bool,
        vat_period -> Text,
        currency -> Text,
        address -> Text,
        city -> Text,
        post_code -> Text,
        phone -> Text,
        email -> Text,
        website -> Text,
        mol -> Text,
        manager_eik -> Text,
        accountant_name -> Text,
        nap_office -> Text,
        iban -> Text,
        bic -> Text,
        inventory_valuation_method -> Text,
        fiscal_year_start_month -> Int4,
        country -> Text,
        manager_name -> Text,
        region -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    company_details (company_id) {
        company_id -> Int8,
        accountant_egn -> Text,
        manager_egn -> Text,
        tax_authority -> Text,
        authorized_person_name -> Text,
        authorized_person_egn -> Text,
        vat_branch_number -> Text,
        tax_accounting_basis -> Text,
        tax_entity -> Text,
        software_company_name -> Text,
        software_id -> Text,
        software_version -> Text,
        is_part_of_group -> Text,
        street_name -> Text,
        building_number -> Text,
        address_building -> Text,
        additional_address_detail -> Text,
        address_type -> Text,
        settings -> Jsonb,
    }
}

diesel::table! {
    user_companies (id) {
        id -> Int8,
        user_id -> Int8,
        company_id -> Int8,
        role_id -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    accounts (id) {
        id -> Int8,
        company_id -> Int8,
        number -> Text,
        name -> Text,
        saft_account_type -> Text,
        analytic_type -> Text,
        is_currency -> Bool,
    }
}

diesel::table! {
    bank_accounts (id) {
        id -> Int8,
        company_id -> Int8,
        iban -> Text,
        bic -> Text,
        bank_name -> Text,
        currency -> Text,
        name -> Text,
        account_number -> Text,
        gl_account_id -> Int8,
        import_format -> Text,
        balance -> Numeric,
    }
}

diesel::table! {
    bank_transactions (id) {
        id -> Int8,
        company_id -> Int8,
        bank_account_id -> Int8,
        tx_date -> Date,
        amount -> Numeric,
        direction -> Text,
        counterpart_name -> Text,
        description -> Text,
        reference -> Text,
        status -> Text,
        journal_entry_id -> Int8,
    }
}

diesel::allow_tables_to_appear_in_same_query!(companies, company_details, user_companies);

diesel::table! {
    counterparts (id) {
        id -> Int8,
        company_id -> Int8,
        name -> Text,
        counterpart_type -> Text,
        eik -> Text,
        vat_number -> Text,
        address -> Text,
        city -> Text,
        post_code -> Text,
        country -> Text,
        contact_person -> Text,
    }
}

diesel::table! {
    counterpart_extra (id) {
        id -> Int8,
        vies_address -> Text,
        street_name -> Text,
        building_number -> Text,
        address_building -> Text,
        additional_address_detail -> Text,
        address_type -> Text,
        region -> Text,
        email -> Text,
        phone -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(counterparts, counterpart_extra);

diesel::table! {
    products (id) {
        id -> Int8,
        company_id -> Int8,
        name -> Text,
        code -> Text,
        description -> Text,
        unit -> Text,
        price -> Numeric,
        cost_price -> Numeric,
        vat_rate -> Numeric,
        is_service -> Bool,
        is_inventory -> Bool,
        inventory_account_id -> Int8,
        revenue_account_id -> Int8,
        stock_quantity -> Numeric,
        stock_value -> Numeric,
        is_active -> Bool,
    }
}

diesel::table! {
    product_extra (id) {
        id -> Int8,
        product_type -> Text,
        cn8_code -> Text,
        cn8_name -> Text,
        expense_account_id -> Int8,
        auto_write_off -> Bool,
    }
}

diesel::allow_tables_to_appear_in_same_query!(products, product_extra);

diesel::table! {
    company_locations (id) {
        id -> Int8,
        company_id -> Int8,
        name -> Text,
        location_type -> Text,
        street_name -> Text,
        building_number -> Text,
        city -> Text,
        post_code -> Text,
        region -> Text,
        country -> Text,
        is_main -> Bool,
    }
}

diesel::table! {
    beneficial_owners (id) {
        id -> Int8,
        company_id -> Int8,
        first_name_bg -> Text,
        last_name_bg -> Text,
        egn -> Text,
        first_name_latin -> Text,
        last_name_latin -> Text,
        country -> Text,
        ownership_percentage -> Text,
    }
}

diesel::table! {
    ultimate_parents (id) {
        id -> Int8,
        company_id -> Int8,
        name_bg -> Text,
        uic -> Text,
        name_latin -> Text,
        country -> Text,
    }
}

diesel::table! {
    document_series (id) {
        id -> Int8,
        company_id -> Int8,
        name -> Text,
        document_types -> Text,
        pad_width -> Int4,
        start_number -> Int4,
        next_number -> Text,
    }
}

diesel::table! {
    invoices (id) {
        id -> Int8,
        company_id -> Int8,
        direction -> Text,
        document_type -> Text,
        number -> Text,
        issue_date -> Date,
        due_date -> Nullable<Date>,
        counterpart_id -> Int8,
        counterpart_name -> Text,
        currency -> Text,
        currency_rate -> Numeric,
        net_amount -> Numeric,
        vat_amount -> Numeric,
        total_amount -> Numeric,
        status -> Text,
        payment_method -> Text,
        notes -> Text,
        accounting_month -> Text,
        original_invoice_id -> Int8,
        journal_entry_id -> Int8,
        tax_event_date -> Nullable<Date>,
        discount_percent -> Numeric,
        discount_amount -> Numeric,
        vat_exemption_reason -> Text,
        location_id -> Int8,
        exclude_vat -> Bool,
        vat_article -> Text,
        customs_statistical -> Text,
        customs_duty -> Numeric,
        customs_vat -> Numeric,
    }
}

diesel::table! {
    invoice_lines (id) {
        id -> Int8,
        invoice_id -> Int8,
        code -> Text,
        description -> Text,
        quantity -> Numeric,
        unit -> Text,
        unit_price -> Numeric,
        vat_rate -> Numeric,
        product_id -> Int8,
        net_amount -> Numeric,
        vat_amount -> Numeric,
        total_amount -> Numeric,
    }
}

diesel::table! {
    journal_entries (id) {
        id -> Int8,
        company_id -> Int8,
        entry_date -> Date,
        document_type -> Text,
        document_id -> Int8,
        document_number -> Text,
        description -> Text,
        vat_type -> Text,
        counterpart_id -> Int8,
        counterpart_name -> Text,
        document_date -> Date,
        vat_period -> Text,
    }
}

diesel::table! {
    journal_lines (id) {
        id -> Int8,
        entry_id -> Int8,
        account_id -> Int8,
        account_number -> Text,
        account_name -> Text,
        direction -> Text,
        amount -> Numeric,
        vat_amount -> Numeric,
        currency -> Text,
        fx_quantity -> Numeric,
        fx_rate -> Numeric,
    }
}

diesel::table! {
    accounting_periods (id) {
        id -> Int8,
        company_id -> Int8,
        year -> Int4,
        month -> Int4,
        status -> Text,
        closed_at -> Nullable<Timestamptz>,
        notes -> Text,
    }
}

diesel::table! {
    opening_balances (id) {
        id -> Int8,
        company_id -> Int8,
        account_id -> Int8,
        year -> Int4,
        debit -> Numeric,
        credit -> Numeric,
    }
}

diesel::allow_tables_to_appear_in_same_query!(journal_entries, journal_lines);

diesel::table! {
    bank_matching_rules (id) {
        id -> Int8,
        company_id -> Int8,
        name -> Text,
        keywords -> Text,
        direction -> Text,
        priority -> Int4,
        is_active -> Bool,
        account_id -> Int8,
        counterpart_id -> Int8,
    }
}

diesel::table! {
    fixed_asset_categories (id) {
        id -> Int8,
        company_id -> Int8,
        name -> Text,
        cita_category -> Text,
        min_depreciation_rate -> Numeric,
        max_depreciation_rate -> Numeric,
        default_method -> Text,
        asset_account_id -> Int8,
        depreciation_account_id -> Int8,
        expense_account_id -> Int8,
    }
}

diesel::table! {
    fixed_assets (id) {
        id -> Int8,
        company_id -> Int8,
        name -> Text,
        inventory_number -> Text,
        category_id -> Int8,
        acquisition_date -> Date,
        put_into_service_date -> Nullable<Date>,
        cost -> Numeric,
        salvage_value -> Numeric,
        useful_life_months -> Int4,
        depreciation_method -> Text,
        accounting_depreciation_rate -> Numeric,
        tax_depreciation_rate -> Numeric,
        accumulated_depreciation -> Numeric,
        tax_accumulated_depreciation -> Numeric,
        status -> Text,
        serial_number -> Text,
        manufacturer -> Text,
        model -> Text,
        responsible_person -> Text,
        is_conserved -> Bool,
        location_id -> Int8,
    }
}

diesel::table! {
    inventory_lots (id) {
        id -> Int8,
        company_id -> Int8,
        product_id -> Int8,
        invoice_id -> Int8,
        invoice_line_id -> Int8,
        received_date -> Date,
        quantity -> Numeric,
        remaining -> Numeric,
        unit_cost -> Numeric,
        location_id -> Int8,
    }
}

diesel::table! {
    stock_documents (id) {
        id -> Int8,
        company_id -> Int8,
        number -> Text,
        doc_date -> Date,
        kind -> Text,
        from_location_id -> Int8,
        to_location_id -> Int8,
        status -> Text,
        notes -> Text,
        client_key -> Nullable<Text>,
    }
}

diesel::table! {
    stock_document_lines (id) {
        id -> Int8,
        document_id -> Int8,
        product_id -> Int8,
        quantity -> Numeric,
        unit_cost -> Numeric,
        description -> Text,
    }
}

diesel::table! {
    dividend_distributions (id) {
        id -> Int8,
        company_id -> Int8,
        year -> Int4,
        total_amount -> Numeric,
        decision_date -> Date,
        decision_number -> Text,
        status -> Text,
        notes -> Text,
    }
}

diesel::table! {
    dividends (id) {
        id -> Int8,
        company_id -> Int8,
        distribution_id -> Int8,
        beneficial_owner_id -> Int8,
        owner_name -> Text,
        owner_egn -> Text,
        ownership_percentage -> Text,
        gross_amount -> Numeric,
        tax_rate -> Numeric,
        tax_amount -> Numeric,
        net_amount -> Numeric,
        decision_date -> Date,
        payment_date -> Nullable<Date>,
        is_paid -> Bool,
        notes -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(dividends, dividend_distributions);

diesel::table! {
    recipes (id) {
        id -> Int8,
        company_id -> Int8,
        code -> Text,
        name -> Text,
        output_product_id -> Int8,
        output_quantity -> Numeric,
        output_unit -> Text,
        notes -> Text,
        is_active -> Bool,
    }
}

diesel::table! {
    recipe_lines (id) {
        id -> Int8,
        recipe_id -> Int8,
        product_id -> Int8,
        quantity -> Numeric,
        unit -> Text,
        wastage_percent -> Numeric,
        line_no -> Int4,
    }
}

diesel::table! {
    production_orders (id) {
        id -> Int8,
        company_id -> Int8,
        number -> Text,
        recipe_id -> Int8,
        output_product_id -> Int8,
        quantity -> Numeric,
        location_id -> Int8,
        order_date -> Date,
        status -> Text,
        notes -> Text,
        material_cost -> Numeric,
        journal_entry_id -> Int8,
    }
}

diesel::table! {
    production_order_lines (id) {
        id -> Int8,
        order_id -> Int8,
        product_id -> Int8,
        product_name -> Text,
        quantity -> Numeric,
        unit -> Text,
        unit_cost -> Numeric,
        amount -> Numeric,
    }
}

diesel::table! {
    roles (id) {
        id -> Int8,
        name -> Text,
        description -> Text,
        permissions -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    audit_logs (id) {
        id -> Int8,
        company_id -> Int8,
        user_id -> Int8,
        user_login -> Text,
        user_name -> Text,
        entity_type -> Text,
        entity_id -> Int8,
        entity_number -> Text,
        action -> Text,
        summary -> Text,
        old_amount -> Text,
        new_amount -> Text,
        old_data -> Nullable<Jsonb>,
        new_data -> Nullable<Jsonb>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    locations (id) {
        id -> Int8,
        company_id -> Int8,
        name -> Text,
        is_active -> Bool,
    }
}

diesel::table! {
    internal_docs (id) {
        id -> Int8,
        company_id -> Int8,
        number -> Text,
        doc_date -> Date,
        from_location_id -> Int8,
        to_location_id -> Int8,
        status -> Text,
        notes -> Text,
        handed_by -> Text,
        received_by -> Text,
    }
}

diesel::table! {
    internal_doc_lines (id) {
        id -> Int8,
        doc_id -> Int8,
        line_kind -> Text,
        product_id -> Int8,
        asset_id -> Int8,
        quantity -> Numeric,
        unit_cost -> Numeric,
        amount -> Numeric,
        description -> Text,
    }
}

diesel::table! {
    material_issues (id) {
        id -> Int8,
        company_id -> Int8,
        number -> Text,
        doc_date -> Date,
        location_id -> Int8,
        status -> Text,
        notes -> Text,
        journal_entry_id -> Int8,
    }
}

diesel::table! {
    material_issue_lines (id) {
        id -> Int8,
        doc_id -> Int8,
        product_id -> Int8,
        quantity -> Numeric,
        unit_cost -> Numeric,
        amount -> Numeric,
        account_id -> Int8,
        inventory_account_id -> Int8,
        line_no -> Int4,
    }
}

diesel::table! {
    stock_account_rules (id) {
        id -> Int8,
        company_id -> Int8,
        kind -> Text,
        inventory_account_id -> Int8,
        offset_account_id -> Int8,
        clearing_account_id -> Int8,
    }
}

diesel::table! {
    fixed_asset_events (id) {
        id -> Int8,
        company_id -> Int8,
        asset_id -> Int8,
        event_type -> Text,
        event_date -> Date,
        amount -> Numeric,
        location -> Text,
        reason -> Text,
    }
}

diesel::table! {
    password_reset_tokens (id) {
        id -> Int8,
        user_id -> Int8,
        token_hash -> Text,
        expires_at -> Int8,
        used -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    exchange_rates (id) {
        id -> Int8,
        currency_code -> Text,
        rate -> Numeric,
        rate_date -> Date,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    fs_line_formulas (id) {
        id -> Int8,
        company_id -> Int8,
        statement -> Text,
        line_code -> Text,
        formula -> Text,
    }
}

diesel::table! {
    saft_account_mappings (id) {
        id -> Int8,
        company_id -> Int8,
        kind -> Text,
        type_code -> Text,
        debit_account -> Text,
        credit_account -> Text,
        debit_analytical -> Text,
        credit_analytical -> Text,
        description -> Text,
        is_active -> Bool,
    }
}
