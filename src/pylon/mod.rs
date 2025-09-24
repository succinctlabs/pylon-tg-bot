mod issue;
use eyre::eyre;
pub use issue::Issue;

mod responses;
pub use responses::SuccessResponse;

use crate::pylon::responses::{CreateIssueResponse, ErrorResponse};

const PYLON_API_URL: &str = "https://api.usepylon.com";

pub struct PylonClient {
    api_token: String,
    http_client: reqwest::Client,
}

impl PylonClient {
    pub fn new(api_token: String) -> Self {
        let http_client = reqwest::Client::new();

        PylonClient {
            api_token,
            http_client,
        }
    }

    pub async fn create_issue(
        &self,
        title: &str,
        body_html: &str,
        account_id: &str,
    ) -> Result<CreateIssueResponse, eyre::Error> {
        let issue = Issue {
            account_id,
            title,
            body_html,
        };

        let response = self
            .http_client
            .post(format!("{PYLON_API_URL}/issues"))
            .bearer_auth(&self.api_token)
            .json(&issue)
            .send()
            .await?;

        match response.status().as_u16() {
            200 => {
                let response = response
                    .json::<SuccessResponse<CreateIssueResponse>>()
                    .await?;
                Ok(response.data)
            }
            _ => {
                let response = response.json::<ErrorResponse>().await?;
                Err(eyre!("{}", response.errors.join(", ")))
            }
        }
    }
}
