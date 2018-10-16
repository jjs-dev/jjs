pub type Token = u64;

#[derive(Debug, Serialize, Deserialize)]
pub enum Auth {
    Guest,
    ByToken(Token),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CheckSumAlgorithm {
    Sha2,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckSum {
    pub algorithm: CheckSumAlgorithm,
    pub digest: String,
}