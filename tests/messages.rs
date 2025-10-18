use std::env;

use pylon_tg_bot::{config::Settings, pylon::PylonClient};

#[tokio::test(flavor = "multi_thread")]
async fn test_get_pylon_account() {
    dotenvy::dotenv().unwrap();

    let settings = confy::load_path::<Settings>("./settings.toml").unwrap();

    let pylon_client = PylonClient::new(env::var("PYLON_API_TOKEN").unwrap());

    for pylon_account_id in settings.tg_chats_to_pylon_accounts.values() {
        if !pylon_account_id.is_empty() {
            let account = pylon_client
                .get_account(pylon_account_id)
                .await
                .unwrap()
                .unwrap();

            println!("{}", account.name.unwrap_or_default())
        }
    }
}
