pub struct ContestId(pub(super) String);

impl ContestId {
    pub fn new(s: String) -> ContestId {
        ContestId(s)
    }
}

pub struct UserId(uuid::Uuid);

impl UserId {
    pub fn new(u: uuid::Uuid) -> UserId {
        UserId(u)
    }
}
