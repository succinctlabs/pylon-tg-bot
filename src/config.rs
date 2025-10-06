use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub struct Config {
    settings: RwLock<Settings>,
    settings_path: String,
}

impl Config {
    pub fn try_new(settings_path: String) -> eyre::Result<Self> {
        let settings = confy::load_path::<Settings>(settings_path.clone())?;

        let settings = Self {
            settings: RwLock::new(settings),
            settings_path,
        };

        Ok(settings)
    }

    pub async fn get(&self) -> Settings {
        self.settings.read().await.clone()
    }

    pub async fn reload(&self) -> eyre::Result<()> {
        *self.settings.write().await = confy::load_path::<Settings>(self.settings_path.clone())?;

        Ok(())
    }

    pub fn save(&self, settings: Settings) -> eyre::Result<()> {
        confy::store_path(&self.settings_path, settings.clone())?;

        Ok(())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub tg_chats_to_pylon_accounts: HashMap<String, String>,
    pub bot_admins: HashSet<String>,
}
