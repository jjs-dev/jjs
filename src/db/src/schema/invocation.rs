use super::{Invocation, InvocationPatch, NewInvocation};
use anyhow::Context;
use std::convert::{TryFrom, TryInto};
#[derive(Copy, Clone, Debug, postgres_types::ToSql, postgres_types::FromSql)]
#[repr(i16)]
pub enum InvocationState {
    Queue = 1,
    InWork,
    Unscheduled,
    JudgeDone,
    CompileError,
    InvokeFailed,
    __Last,
}

impl InvocationState {
    pub fn is_finished(self) -> bool {
        match self {
            InvocationState::Queue | InvocationState::InWork | InvocationState::Unscheduled => {
                false
            }
            InvocationState::JudgeDone
            | InvocationState::CompileError
            | InvocationState::InvokeFailed => true,
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
            outcome: serde_json::Value::Array(vec![]),
        })
    }
}

impl Invocation {
    pub fn invoke_task(&self) -> anyhow::Result<invoker_api::DbInvokeTask> {
        Ok(bincode::deserialize(&self.invoke_task).context("invalid InvokeTaslk")?)
    }

    pub fn invoke_outcome_headers(&self) -> anyhow::Result<Vec<invoker_api::InvokeOutcomeHeader>> {
        Ok(serde_json::from_value(self.outcome.clone()).context("invalid InvokeOutcomeHeader")?)
    }

    pub fn state(&self) -> anyhow::Result<InvocationState> {
        Ok(self.state.try_into().context("invalid InvocationState")?)
    }
}

impl Invocation {
    pub(crate) fn from_pg_row(row: tokio_postgres::Row) -> Invocation {
        Self {
            id: row.get("id"),
            run_id: row.get("run_id"),
            state: row.get("state"),
            outcome: row.get("outcome"),
            invoke_task: row.get("invoke_task"),
        }
    }
}

impl InvocationPatch {
    pub fn state(&mut self, state: InvocationState) -> &mut Self {
        self.state = Some(state.into());
        self
    }
}
