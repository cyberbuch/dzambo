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
    invoices (id) {
        id -> Int8,
        company_id -> Int8,
        number -> Text,
        issue_date -> Date,
        counterpart_id -> Int8,
        counterpart_name -> Text,
        currency -> Text,
        net_amount -> Numeric,
        vat_amount -> Numeric,
        total_amount -> Numeric,
        status -> Text,
        direction -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    bank_accounts (id) {
        id -> Int8,
        company_id -> Int8,
        balance -> Numeric,
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
