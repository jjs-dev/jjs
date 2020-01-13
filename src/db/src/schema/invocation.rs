use super::{Invocation, InvocationPatch, NewInvocation};
use anyhow::Context;
use std::convert::{TryFrom, TryInto};
#[derive(Copy, Clone, AsExpression)]
#[repr(i16)]
pub enum InvocationState {
    Queue = 1,
    Execute,
    Unscheduled,
    Done,
    __Last,
}

impl InvocationState {
    pub fn is_finished(self) -> bool {
        match self {
            InvocationState::Queue | InvocationState::Execute | InvocationState::Unscheduled => {
                false
            }
            InvocationState::Done => true,
            InvocationState::__Last => unreachable!(),
        }
    }

    pub const fn as_int(self) -> i16 {
        self as i16
    }
}

impl From<InvocationState> for i16 {
    fn from(is: InvocationState) -> i16 {
        is.as_int()
    }
}

#[derive(Debug)]
pub struct UnknownInvocationStateError;

impl std::fmt::Display for UnknownInvocationStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "unknown invocation state")
    }
}

impl std::error::Error for UnknownInvocationStateError {}

impl TryFrom<i16> for InvocationState {
    type Error = UnknownInvocationStateError;

    fn try_from(d: i16) -> Result<InvocationState, UnknownInvocationStateError> {
        if d < 0 || d >= (InvocationState::__Last as i16) {
            return Err(UnknownInvocationStateError);
        }
        Ok(unsafe { std::mem::transmute(d) })
    }
}

impl NewInvocation {
    pub fn new(invoke_task: &invoker_api::DbInvokeTask) -> anyhow::Result<NewInvocation> {
        Ok(NewInvocation {
            invoke_task: bincode::serialize(invoke_task)?,
            run_id: invoke_task.run_id as i32,
            state: InvocationState::Queue.into(),
            outcome: serde_json::to_value(invoker_api::InvokeOutcomeHeader {
                score: None,
                status: None,
            })?,
        })
    }
}

impl Invocation {
    pub fn invoke_task(&self) -> anyhow::Result<invoker_api::DbInvokeTask> {
        Ok(bincode::deserialize(&self.invoke_task).context("invalid InvokeTaslk")?)
    }

    pub fn invoke_outcome_header(&self) -> anyhow::Result<invoker_api::InvokeOutcomeHeader> {
        Ok(serde_json::from_value(self.outcome.clone()).context("invalid InvokeOutcomeHeader")?)
    }

    pub fn state(&self) -> anyhow::Result<InvocationState> {
        Ok(self.state.try_into().context("invalid InvocationState")?)
    }
}

impl InvocationPatch {
    pub fn state(&mut self, state: InvocationState) -> &mut Self {
        self.state = Some(state.into());
        self
    }

    pub fn outcome(
        &mut self,
        header: invoker_api::InvokeOutcomeHeader,
    ) -> anyhow::Result<&mut Self> {
        self.outcome = Some(serde_json::to_value(&header).context("failed to serialize header")?);
        Ok(self)
    }
}
