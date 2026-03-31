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

pub fn print_account(account: &Account, accessible: bool, mode: &OutputMode) {
    match mode {
        OutputMode::Json => {
            #[derive(Serialize)]
            struct AccountWithAccessible<'a> {
                #[serde(flatten)]
                account: &'a Account,
                accessible: bool,
            }
            print_json(&AccountWithAccessible { account, accessible });
        }
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
            if let Some(pk) = &account.p256_public_key {
                table.add_row(vec!["P256 Public Key", pk]);
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
            if let Some(ks) = &account.key_storage {
                table.add_row(vec!["Key Storage", ks]);
            }
            table.add_row(vec!["Accessible", if accessible { "yes" } else { "no" }]);
            table.add_row(vec!["Created At", &account.created_at]);
            println!("{table}");
        }
    }
}

pub fn print_account_list(
    list: &AccountList,
    accessibility: &std::collections::HashMap<String, bool>,
    all: bool,
    mode: &OutputMode,
) {
    match mode {
        OutputMode::Json => {
            if all {
                #[derive(Serialize)]
                struct AccountWithAccessible<'a> {
                    #[serde(flatten)]
                    account: &'a Account,
                    accessible: bool,
                }
                #[derive(Serialize)]
                struct AccountListWithAccessible<'a> {
                    accounts: Vec<AccountWithAccessible<'a>>,
                }
                let annotated = AccountListWithAccessible {
                    accounts: list
                        .accounts
                        .iter()
                        .map(|a| AccountWithAccessible {
                            account: a,
                            accessible: accessibility.get(&a.account_id).copied().unwrap_or(false),
                        })
                        .collect(),
                };
                print_json(&annotated);
            } else {
                print_json(list);
            }
        }
        OutputMode::Quiet => {
            for a in &list.accounts {
                println!("{}", a.account_id);
            }
        }
        OutputMode::Table => {
            let mut table = Table::new();
            if all {
                table.set_header(vec![
                    "Account ID",
                    "Signer Type",
                    "Ethereum Signer",
                    "Key Storage",
                    "Accessible",
                    "Created At",
                ]);
                for a in &list.accounts {
                    let accessible = accessibility.get(&a.account_id).copied().unwrap_or(false);
                    let accessible_str = if accessible { "yes" } else { "no" };
                    table.add_row(vec![
                        &a.account_id,
                        a.signer_type.as_deref().unwrap_or("-"),
                        a.ethereum_signer_address.as_deref().unwrap_or("-"),
                        a.key_storage.as_deref().unwrap_or("-"),
                        accessible_str,
                        &a.created_at,
                    ]);
                }
            } else {
                table.set_header(vec![
                    "Account ID",
                    "Signer Type",
                    "Ethereum Signer",
                    "Key Storage",
                    "Created At",
                ]);
                for a in &list.accounts {
                    table.add_row(vec![
                        &a.account_id,
                        a.signer_type.as_deref().unwrap_or("-"),
                        a.ethereum_signer_address.as_deref().unwrap_or("-"),
                        a.key_storage.as_deref().unwrap_or("-"),
                        &a.created_at,
                    ]);
                }
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
