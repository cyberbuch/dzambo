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
