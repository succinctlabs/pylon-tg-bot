use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub tg_chats_to_pylon_accounts: HashMap<String, String>,
    pub bot_admins: HashSet<String>,
}
