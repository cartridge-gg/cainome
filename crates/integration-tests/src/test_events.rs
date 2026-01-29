use katana_runner::RunnerCtx;

use starknet_types_core::felt::Felt;

#[allow(unused)]
use crate::bindings::components_events_flat::*;

#[allow(unused)]
const UDC_ADDRESS: Felt =
    Felt::from_hex_unwrap("0x41a78e741e5af2fec34b695679bc6891742439f7afb8484ecd7766661ad02bf");

#[tokio::test]
#[katana_runner::test(accounts = 2, fee = false, block_time = 1)]
async fn test_events_emission(_runner: &RunnerCtx) {
    // let account = runner.account(0);

    // let path = std::path::Path::new("./src/bindings/simple_get_set.json");
}
