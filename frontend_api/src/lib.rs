#[macro_use]
extern crate serde_derive;

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

//#[derive(Debug, Serialize, Deserialize)]
pub type PingResult = Result<PingSuccess, PingFail>;

#[derive(Debug, Serialize, Deserialize)]
pub enum RequestBody {
    Ping(PingRequest),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Auth {
    Guest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub query: RequestBody,
    pub auth: Auth,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ResponseBody {
    Ping(PingResult),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub result: ResponseBody,
}