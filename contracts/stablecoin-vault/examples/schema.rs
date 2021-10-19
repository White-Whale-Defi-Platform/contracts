use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use stablecoin_vault::pool_info::PoolInfo;
use stablecoin_vault::state::State;
use white_whale::msg::VaultQueryMsg;
use white_whale::ust_vault::msg::{ExecuteMsg, InitMsg, PoolResponse};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(VaultQueryMsg), &out_dir);
    export_schema(&schema_for!(State), &out_dir);
    export_schema(&schema_for!(PoolResponse), &out_dir);
    export_schema(&schema_for!(PoolInfo), &out_dir);
}
