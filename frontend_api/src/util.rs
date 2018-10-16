#[derive(Debug, Serialize, Deserialize)]
pub struct PingRequest {
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PingSuccess {
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PingFail {}

pub type PingResult = Result<PingSuccess, PingFail>;
