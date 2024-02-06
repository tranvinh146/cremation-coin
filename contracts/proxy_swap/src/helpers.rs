use cremation_token::msg::{AssetInfo, RouterExecuteMsg, SwapOperation};

pub fn create_swap_operations(
    offer_asset: AssetInfo,
    ask_asset: AssetInfo,
    swap_paths: Vec<AssetInfo>,
) -> RouterExecuteMsg {
    let mut operations = vec![];
    for i in 0..=swap_paths.len() {
        let offer_asset_info = if i == 0 {
            offer_asset.clone()
        } else {
            swap_paths[i - 1].clone()
        };

        let ask_asset_info = if i == swap_paths.len() {
            ask_asset.clone()
        } else {
            swap_paths[i].clone()
        };

        operations.push(SwapOperation::TerraPort {
            offer_asset_info,
            ask_asset_info,
        });
    }

    RouterExecuteMsg::ExecuteSwapOperations {
        operations,
        to: None,
        minimum_receive: None,
        deadline: None,
    }
}
