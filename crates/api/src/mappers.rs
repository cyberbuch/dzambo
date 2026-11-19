//! Моделите на storage → DTO-та на core (суми/дати като низове).

use chrono::SecondsFormat;

use domain::dto::{
    AccountDto, CompanyDto, CompanySettingsDto, CounterpartDto, DashboardDto, LocationDto, MeDto,
    MembershipDto, OwnerDto, ParentDto, ProductDto, RecentInvoiceDto, SeriesDto,
};
use storage::{
    companies::Membership,
    dashboard::DashboardNumbers,
    models::{invoices::Invoice, User},
};

fn iso(ts: &chrono::DateTime<chrono::Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn company_dto(row: &storage::companies::CompanyRow) -> CompanyDto {
    let c = &row.0;
    let d = row.1.clone().unwrap_or_default();
    CompanyDto {
        id: c.id,
        name: c.name.clone(),
        eik: c.eik.clone(),
        vat_number: c.vat_number.clone(),
        is_vat_registered: c.is_vat_registered,
        vat_period: c.vat_period.clone(),
        currency: c.currency.clone(),
        address: c.address.clone(),
        city: c.city.clone(),
        post_code: c.post_code.clone(),
        phone: c.phone.clone(),
        email: c.email.clone(),
        website: c.website.clone(),
        mol: c.mol.clone(),
        manager_eik: c.manager_eik.clone(),
        accountant_name: c.accountant_name.clone(),
        accountant_egn: d.accountant_egn,
        tax_authority: d.tax_authority,
        nap_office: c.nap_office.clone(),
        iban: c.iban.clone(),
        bic: c.bic.clone(),
        inventory_valuation_method: c.inventory_valuation_method.clone(),
        fiscal_year_start_month: c.fiscal_year_start_month,
        country: c.country.clone(),
        manager_name: c.manager_name.clone(),
        manager_egn: d.manager_egn,
        authorized_person_name: d.authorized_person_name,
        authorized_person_egn: d.authorized_person_egn,
        vat_branch_number: d.vat_branch_number,
        tax_accounting_basis: d.tax_accounting_basis,
        tax_entity: d.tax_entity,
        software_company_name: d.software_company_name,
        software_id: d.software_id,
        software_version: d.software_version,
        is_part_of_group: d.is_part_of_group,
        street_name: d.street_name,
        building_number: d.building_number,
        address_building: d.address_building,
        additional_address_detail: d.additional_address_detail,
        address_type: d.address_type,
        region: c.region.clone(),
        created_at: iso(&c.created_at),
    }
}

pub fn membership_dto(m: &Membership) -> MembershipDto {
    MembershipDto {
        company_id: m.company_id,
        role_id: m.role_id,
        role: None,
        company_name: m.company_name.clone(),
    }
}

pub fn me_dto(u: &User, memberships: &[Membership]) -> MeDto {
    MeDto {
        sub: u.email.clone(),
        id: u.id,
        email: u.email.clone(),
        username: u.email.clone(),
        name: u.name.clone(),
        is_super_admin: u.is_super_admin,
        role_id: u.role_id,
        totp_enabled: u.totp_enabled,
        permissions: if u.is_super_admin {
            vec!["*".to_string()]
        } else {
            Vec::new()
        },
        companies: memberships.iter().map(membership_dto).collect(),
    }
}

pub fn recent_dto(r: &Invoice) -> RecentInvoiceDto {
    RecentInvoiceDto {
        id: r.id,
        number: r.number.clone(),
        issue_date: r.issue_date.to_string(),
        counterpart_id: r.counterpart_id,
        counterpart_name: r.counterpart_name.clone(),
        currency: r.currency.clone(),
        total_amount: domain::dto::fmt_money(&r.total_amount),
        status: r.status.clone(),
    }
}

pub fn dashboard_dto(company_id: i64, n: DashboardNumbers) -> DashboardDto {
    DashboardDto {
        company_id,
        year: n.year,
        invoices_draft: n.invoices_draft,
        invoices_posted: n.invoices_posted,
        receivables: domain::dto::fmt_money(&n.receivables),
        payables: domain::dto::fmt_money(&n.payables),
        vat_output: domain::dto::fmt_money(&n.vat_output),
        vat_input: domain::dto::fmt_money(&n.vat_input),
        revenue_ytd: domain::dto::fmt_money(&n.revenue_ytd),
        expense_ytd: domain::dto::fmt_money(&n.expense_ytd),
        bank_total: domain::dto::fmt_money(&n.bank_total),
        recent: n.recent.iter().map(recent_dto).collect(),
    }
}

pub fn account_dto(a: &storage::models::Account) -> AccountDto {
    AccountDto {
        id: a.id,
        number: a.number.clone(),
        name: a.name.clone(),
        saft_account_type: a.saft_account_type.clone(),
        analytic_type: a.analytic_type.clone(),
        is_currency: a.is_currency,
    }
}

pub fn counterpart_dto(row: &storage::counterparts::CounterpartRow) -> CounterpartDto {
    let c = &row.0;
    let e = &row.1;
    CounterpartDto {
        id: c.id,
        name: c.name.clone(),
        counterpart_type: c.counterpart_type.clone(),
        eik: c.eik.clone(),
        vat_number: c.vat_number.clone(),
        address: c.address.clone(),
        city: c.city.clone(),
        post_code: c.post_code.clone(),
        country: c.country.clone(),
        contact_person: c.contact_person.clone(),
        vies_address: e.vies_address.clone(),
        street_name: e.street_name.clone(),
        building_number: e.building_number.clone(),
        address_building: e.address_building.clone(),
        additional_address_detail: e.additional_address_detail.clone(),
        address_type: e.address_type.clone(),
        region: e.region.clone(),
        email: e.email.clone(),
        phone: e.phone.clone(),
    }
}

fn money_opt(v: &bigdecimal::BigDecimal) -> Option<String> {
    Some(domain::dto::fmt_money(v))
}

pub fn product_dto(row: &storage::products::ProductRow) -> ProductDto {
    let p = &row.0;
    let e = &row.1;
    ProductDto {
        id: p.id,
        name: p.name.clone(),
        code: p.code.clone(),
        description: p.description.clone(),
        unit: p.unit.clone(),
        price: money_opt(&p.price),
        cost_price: money_opt(&p.cost_price),
        vat_rate: money_opt(&p.vat_rate),
        is_service: p.is_service,
        is_inventory: p.is_inventory,
        inventory_account_id: p.inventory_account_id,
        revenue_account_id: p.revenue_account_id,
        stock_quantity: money_opt(&p.stock_quantity),
        stock_value: money_opt(&p.stock_value),
        is_active: p.is_active,
        product_type: e.product_type.clone(),
        cn8_code: e.cn8_code.clone(),
        cn8_name: e.cn8_name.clone(),
        expense_account_id: e.expense_account_id,
        auto_write_off: e.auto_write_off,
    }
}

pub fn location_dto(l: &storage::models::related::Location) -> LocationDto {
    LocationDto {
        id: l.id,
        name: l.name.clone(),
        location_type: l.location_type.clone(),
        street_name: l.street_name.clone(),
        building_number: l.building_number.clone(),
        city: l.city.clone(),
        post_code: l.post_code.clone(),
        region: l.region.clone(),
        country: l.country.clone(),
        is_main: l.is_main,
    }
}

pub fn owner_dto(o: &storage::models::related::BeneficialOwner) -> OwnerDto {
    OwnerDto {
        id: o.id,
        first_name_bg: o.first_name_bg.clone(),
        last_name_bg: o.last_name_bg.clone(),
        egn: o.egn.clone(),
        first_name_latin: o.first_name_latin.clone(),
        last_name_latin: o.last_name_latin.clone(),
        country: o.country.clone(),
        ownership_percentage: o.ownership_percentage.clone(),
    }
}

pub fn parent_dto(p: &storage::models::related::UltimateParent) -> ParentDto {
    ParentDto {
        id: p.id,
        name_bg: p.name_bg.clone(),
        uic: p.uic.clone(),
        name_latin: p.name_latin.clone(),
        country: p.country.clone(),
    }
}

pub fn series_dto(s: &storage::models::related::DocumentSeries) -> SeriesDto {
    SeriesDto {
        id: s.id,
        name: s.name.clone(),
        document_types: s.document_types.clone(),
        pad_width: s.pad_width,
        start_number: s.start_number,
        next_number: s.next_number.clone(),
    }
}

pub fn settings_dto(
    company: &CompanyDto,
    locations: Vec<storage::models::related::Location>,
    owners: Vec<storage::models::related::BeneficialOwner>,
    parents: Vec<storage::models::related::UltimateParent>,
    series: Vec<storage::models::related::DocumentSeries>,
) -> CompanySettingsDto {
    CompanySettingsDto {
        company: company.clone(),
        locations: locations.iter().map(location_dto).collect(),
        beneficial_owners: owners.iter().map(owner_dto).collect(),
        ultimate_parents: parents.iter().map(parent_dto).collect(),
        document_series: series.iter().map(series_dto).collect(),
    }
}
