//! Interprets given request graph
use judging_apis::invoke::InvokeRequest;

pub struct Interpreter<'a> {
    req: &'a InvokeRequest,
}

impl<'a> Interpreter<'a> {
    pub fn new(req: &'a InvokeRequest) -> Self {
        Interpreter { req }
    }
}
