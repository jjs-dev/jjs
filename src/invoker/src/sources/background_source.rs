/// This is common facade of all push-based task sources (currently
/// all sources except DbSource are push-based)
use crate::controller::{InvocationFinishReason, TaskSource};
use invoker_api::InvokeTask;
use serde::Serialize;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Stores all the data
struct Inner {
    queue: VecDeque<InvokeTask>,
    // for each handle, we store queue of Messages that must be delivered
    // to this handle
    messages: HashMap<Handle, VecDeque<Message>>,
    invocation_owner: HashMap<Uuid, Handle>,
    next_handle: u8,
}

impl Inner {
    fn next_handle(&mut self) -> Handle {
        assert!(self.next_handle != u8::max_value());
        let h = Handle(self.next_handle);
        self.next_handle += 1;
        h
    }
}

impl Inner {
    fn new() -> Inner {
        Inner {
            queue: VecDeque::new(),
            messages: HashMap::new(),
            next_handle: 0,
            invocation_owner: HashMap::new(),
        }
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Handle(u8);

/// Wrapper for underlying state, supporting creation of new
/// `BackgroundSourc`es
pub struct BackgroundSourceManager(Arc<Mutex<Inner>>);

impl BackgroundSourceManager {
    pub fn create() -> BackgroundSourceManager {
        let state = Inner::new();
        let state = Arc::new(Mutex::new(state));
        BackgroundSourceManager(state)
    }

    pub async fn fork(&self) -> BackgroundSourceHandle {
        let handle = self.0.lock().await.next_handle();
        BackgroundSourceHandle {
            state: self.0.clone(),
            handle,
        }
    }

    /// When all Handles are created, this method trabsforms
    /// Manager into BackgroundSource, which can be later passed
    /// to controller.
    pub fn into_source(self) -> BackgroundSource {
        BackgroundSource(self.0)
    }
}

/// Provides API for controller
/// Implemented as handle to data storage, unique for each underlying source
pub struct BackgroundSource(Arc<Mutex<Inner>>);

impl BackgroundSource {
    async fn push_message(&self, msg: Message) {
        let mut st = self.0.lock().await;
        let handle = st.invocation_owner[&msg.get_id()].clone();
        st.messages.entry(handle).or_default().push_back(msg);
    }
}

/// Provides API for task provider
/// Implemented as handle to data storage, unique for each underlying source.
#[derive(Clone)]
pub struct BackgroundSourceHandle {
    state: Arc<Mutex<Inner>>,
    handle: Handle,
}

impl BackgroundSourceHandle {
    pub async fn add_task(&self, task: InvokeTask) {
        let mut st = self.state.lock().await;
        let prev = st
            .invocation_owner
            .insert(task.invocation_id, self.handle.clone());
        assert!(prev.is_none());
        st.queue.push_back(task);
    }

    pub async fn pop_msg(&self) -> Option<Message> {
        let mut st = self.state.lock().await;

        st.messages
            .get_mut(&self.handle)
            .and_then(|q| q.pop_front())
    }
}

#[derive(Serialize)]
pub enum Message {
    Finish(FinishedMessage),
    Progress(ProgressMessage),
    LiveStatusUpdate(LsuMessage),
}

impl Message {
    fn get_id(&self) -> Uuid {
        match self {
            Message::Finish(inner) => inner.invocation_id,
            Message::LiveStatusUpdate(inner) => inner.invocation_id,
            Message::Progress(inner) => inner.invocation_id,
        }
    }
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
    update: invoker_api::LiveStatusUpdate,
}

#[async_trait::async_trait]
impl TaskSource for BackgroundSource {
    async fn load_tasks(&self, cnt: usize) -> anyhow::Result<Vec<InvokeTask>> {
        let mut q = self.0.lock().await;
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
        self.push_message(Message::Finish(msg)).await;
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
        self.push_message(Message::Progress(msg)).await;
        Ok(())
    }

    async fn deliver_live_status_update(
        &self,
        invocation_id: Uuid,
        update: invoker_api::LiveStatusUpdate,
    ) -> anyhow::Result<()> {
        let msg = LsuMessage {
            update,
            invocation_id,
        };
        self.push_message(Message::LiveStatusUpdate(msg)).await;
        Ok(())
    }
}
