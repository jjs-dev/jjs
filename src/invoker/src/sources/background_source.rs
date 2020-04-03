use crate::controller::{InvocationFinishReason, TaskSource};
use invoker_api::InvokeTask;
use serde::Serialize;
use std::{collections::VecDeque, };
use uuid::Uuid;
use tokio::sync::Mutex;

struct BackgroundSourceState {
    queue: VecDeque<InvokeTask>,
    messages: VecDeque<Message>,
}
pub struct BackgroundSource {
    state: Mutex<BackgroundSourceState>,
}

impl BackgroundSource {
    pub fn new() -> BackgroundSource {
        let state = BackgroundSourceState {
            queue: VecDeque::new(),
            messages: VecDeque::new(),
        };
        let state = Mutex::new(state);
        BackgroundSource { state }
    }

    pub async fn add_task(&self, task: InvokeTask) {
        let mut st = self.state.lock().await;
        st.queue.push_back(task);
    }

    pub async fn pop_msg(&self) -> Option<Message> {
        let mut st = self.state.lock().await;
        st.messages.pop_front()
    }
}

impl Default for BackgroundSource {
    fn default() -> BackgroundSource {
        BackgroundSource::new()
    }
}

#[derive(Serialize)]
pub enum Message {
    Finish(FinishedMessage),
    Progress(ProgressMessage),
    LiveStatusUpdate(LsuMessage)
}
#[derive(Serialize)]
pub struct FinishedMessage {
    invocation_id: Uuid,
    reason: &'static str,
}

#[derive(Serialize)]
pub struct ProgressMessage {
    invocation_id: Uuid,
    header: invoker_api::InvokeOutcomeHeader,
}

#[derive(Serialize)]
pub struct LsuMessage {
    invocation_id: Uuid,
    update: invoker_api::LiveStatusUpdate
}

#[async_trait::async_trait]
impl TaskSource for BackgroundSource {
    async fn load_tasks(&self, cnt: usize) -> anyhow::Result<Vec<InvokeTask>> {
        let mut q = self.state.lock().await;
        let q = &mut q.queue;
        Ok(q.drain(0..cnt.min(q.len())).collect())
    }

    async fn set_finished(
        &self,
        invocation_id: Uuid,
        reason: InvocationFinishReason,
    ) -> anyhow::Result<()> {
        let reason = match reason {
            InvocationFinishReason::CompileError => "CompileError",
            InvocationFinishReason::JudgeDone => "JudgeDone",
            InvocationFinishReason::Fault => "Fault",
        };
        let msg = FinishedMessage {
            reason,
            invocation_id,
        };
        self.state.lock().await.messages.push_back(Message::Finish(msg));
        Ok(())
    }

    async fn add_outcome_header(
        &self,
        invocation_id: Uuid,
        header: invoker_api::InvokeOutcomeHeader,
    ) -> anyhow::Result<()> {
        let msg = ProgressMessage {
            invocation_id,
            header,
        };
        self.state.lock().await.messages.push_back(Message::Progress(msg));
        Ok(())
    }

    async fn deliver_live_status_update(&self, invocation_id: Uuid, update: invoker_api::LiveStatusUpdate) -> anyhow::Result<()> {
        let msg = LsuMessage {
            update,
            invocation_id
        };
        self.state.lock().await.messages.push_back(Message::LiveStatusUpdate(msg));
        Ok(())
    }
}
