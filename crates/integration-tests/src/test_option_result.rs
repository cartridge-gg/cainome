use cainome_rs_macro::abigen;

abigen!(
    MyContract,
    "./contracts/abi/option_result.abi.json",
    cainome_serde_path("cainome_cairo_serde"),
    root_module_path("crate::test_option_result"),
    generic_resolver {
        "contracts::abicov::option_result::option_result::GenericOne" -> "a" = "A";
    }
);

#[tokio::test]
async fn main() {}
