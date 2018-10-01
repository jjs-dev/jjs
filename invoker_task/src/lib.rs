extern crate serde;

#[macro_use]
extern crate serde_derive;



#[derive(Serialize, Deserialize, Debug)]
pub struct ExitRequest {

}

#[derive(Serialize, Deserialize, Debug)]
pub struct PrintRequest {
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub  struct NoopRequest {

}

#[derive(Serialize, Deserialize, Debug)]
pub struct InvokeRequest {
    pub submission_name: String,
    pub toolchain_name: String,
    //pub mask: Mask,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Exit(ExitRequest),
    Print(PrintRequest),
    Invoke(InvokeRequest),
    Noop(NoopRequest),
}