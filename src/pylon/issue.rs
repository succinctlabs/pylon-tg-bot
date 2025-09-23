use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Issue<'a> {
    pub account_id: &'a str,
    pub title: &'a str,
    pub body_html: &'a str,
}
