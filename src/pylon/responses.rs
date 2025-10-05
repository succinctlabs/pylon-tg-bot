use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SuccessResponse<T> {
    pub data: T,
    pub request_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateIssueResponse {
    pub id: Option<String>,
    pub number: Option<u64>,
    pub link: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetAccountResponse {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub errors: Vec<String>,
    pub exists_id: Option<String>,
    pub request_id: Option<String>,
}
