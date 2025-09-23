mod issue;
pub use issue::Issue;

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
    ) -> Result<(), reqwest::Error> {
        let issue = Issue {
            account_id,
            title,
            body_html,
        };

        let _ = self
            .http_client
            .post(format!("{PYLON_API_URL}/issues"))
            .bearer_auth(&self.api_token)
            .json(&issue)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
