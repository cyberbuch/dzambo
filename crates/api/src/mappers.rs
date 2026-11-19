//! Моделите на storage → DTO-та на core (суми/дати като низове).

use chrono::SecondsFormat;

use domain::dto::{CompanyDto, DashboardDto, MeDto, MembershipDto, RecentInvoiceDto};
use storage::{
    companies::Membership,
    dashboard::DashboardNumbers,
    models::{InvoiceSummary, User},
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

pub fn recent_dto(r: &InvoiceSummary) -> RecentInvoiceDto {
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
