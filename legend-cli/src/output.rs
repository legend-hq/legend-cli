use std::io::IsTerminal;

use comfy_table::Table;
use legend_client::types::*;
use serde::Serialize;

#[derive(Clone)]
pub enum OutputMode {
    Json,
    Table,
    Quiet,
}

pub fn detect_mode(json_flag: bool, quiet_flag: bool) -> OutputMode {
    if quiet_flag {
        OutputMode::Quiet
    } else if json_flag || !std::io::stdout().is_terminal() {
        OutputMode::Json
    } else {
        OutputMode::Table
    }
}

pub fn print_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).unwrap());
}

pub fn print_account(account: &Account, mode: &OutputMode) {
    match mode {
        OutputMode::Json => print_json(account),
        OutputMode::Quiet => println!("{}", account.account_id),
        OutputMode::Table => {
            let mut table = Table::new();
            table.set_header(vec!["Field", "Value"]);
            table.add_row(vec!["Account ID", &account.account_id]);
            if let Some(st) = &account.signer_type {
                table.add_row(vec!["Signer Type", st]);
            }
            if let Some(addr) = &account.ethereum_signer_address {
                table.add_row(vec!["Ethereum Signer", addr]);
            }
            if let Some(addr) = &account.legend_wallet_address {
                table.add_row(vec!["Legend Wallet", addr]);
            }
            if let Some(addr) = &account.solana_wallet_address {
                table.add_row(vec!["Solana Wallet", addr]);
            }
            if let Some(org) = &account.turnkey_sub_org_id {
                table.add_row(vec!["Turnkey Sub-Org", org]);
            }
            table.add_row(vec!["Created At", &account.created_at]);
            println!("{table}");
        }
    }
}

pub fn print_account_list(list: &AccountList, mode: &OutputMode) {
    match mode {
        OutputMode::Json => print_json(list),
        OutputMode::Quiet => {
            for a in &list.accounts {
                println!("{}", a.account_id);
            }
        }
        OutputMode::Table => {
            let mut table = Table::new();
            table.set_header(vec![
                "Account ID",
                "Signer Type",
                "Ethereum Signer",
                "Created At",
            ]);
            for a in &list.accounts {
                table.add_row(vec![
                    &a.account_id,
                    a.signer_type.as_deref().unwrap_or("-"),
                    a.ethereum_signer_address.as_deref().unwrap_or("-"),
                    &a.created_at,
                ]);
            }
            println!("{table}");
        }
    }
}

pub fn print_plan(plan: &Plan, mode: &OutputMode) {
    match mode {
        OutputMode::Json => print_json(plan),
        OutputMode::Quiet => println!("{}", plan.plan_id),
        OutputMode::Table => {
            let mut table = Table::new();
            table.set_header(vec!["Field", "Value"]);
            table.add_row(vec!["Plan ID", &plan.plan_id]);
            table.add_row(vec!["Expires At", &plan.expires_at]);
            if let Some(digest) = plan.digest() {
                table.add_row(vec!["Digest", digest]);
            }
            println!("{table}");
        }
    }
}

pub fn print_execute_result(result: &ExecuteResult, mode: &OutputMode) {
    match mode {
        OutputMode::Json => print_json(result),
        OutputMode::Quiet => println!("{}", result.plan_id),
        OutputMode::Table => {
            let mut table = Table::new();
            table.set_header(vec!["Field", "Value"]);
            table.add_row(vec!["Plan ID", &result.plan_id]);
            if let Some(ref activity_id) = result.activity_id {
                table.add_row(vec!["Activity ID", activity_id]);
            }
            table.add_row(vec!["Status", &result.status]);
            println!("{table}");
        }
    }
}

pub fn print_networks(networks: &NetworkList, mode: &OutputMode) {
    match mode {
        OutputMode::Json => print_json(networks),
        OutputMode::Quiet => {
            for n in &networks.networks {
                println!("{}", n.name);
            }
        }
        OutputMode::Table => {
            let mut table = Table::new();
            table.set_header(vec!["Name", "Chain ID", "Display Name"]);
            for n in &networks.networks {
                table.add_row(vec![&n.name, &n.chain_id.to_string(), &n.display_name]);
            }
            println!("{table}");
        }
    }
}

pub fn print_assets(assets: &AssetMap, mode: &OutputMode) {
    match mode {
        OutputMode::Json => print_json(assets),
        _ => print_json(assets),
    }
}

pub fn print_folio(folio: &Folio, mode: &OutputMode) {
    match mode {
        OutputMode::Json => print_json(folio),
        _ => print_json(folio),
    }
}

pub fn print_activities(list: &ActivityList, mode: &OutputMode) {
    match mode {
        OutputMode::Json => print_json(list),
        OutputMode::Quiet => {
            for a in &list.activities {
                println!("{}", a.id);
            }
        }
        OutputMode::Table => {
            let mut table = Table::new();
            table.set_header(vec!["ID", "Status"]);
            for a in &list.activities {
                table.add_row(vec![&a.id.to_string(), a.status.as_deref().unwrap_or("-")]);
            }
            println!("{table}");
        }
    }
}

pub fn print_activity(activity: &Activity, mode: &OutputMode) {
    match mode {
        OutputMode::Json => print_json(activity),
        OutputMode::Quiet => println!("{}", activity.id),
        OutputMode::Table => print_json(activity),
    }
}

pub fn print_prime_account(pa: &PrimeAccount, mode: &OutputMode) {
    match mode {
        OutputMode::Json => print_json(pa),
        OutputMode::Quiet => println!("{}", pa.id),
        OutputMode::Table => {
            let mut table = Table::new();
            table.set_header(vec!["Field", "Value"]);
            table.add_row(vec!["ID", &pa.id]);
            if let Some(name) = &pa.name {
                table.add_row(vec!["Name", name]);
            }
            if let Some(email) = &pa.email {
                table.add_row(vec!["Email", email]);
            }
            println!("{table}");
        }
    }
}
