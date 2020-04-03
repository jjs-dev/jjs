//! Implements Notifications - messages about run testing updates
use log::{debug, warn};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use crate::controller::TaskSource;
pub(crate) struct Notifier {
    score: Option<u32>,
    test: Option<u32>,
    throttled_until: Instant,
    errored: bool,
    source: Arc<dyn TaskSource>,
    invocation_id: uuid::Uuid,
}

impl std::fmt::Debug for Notifier {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Notifier")
        .field("score", &self.score)
        .field("test", &self.test)
        .field("throttled_until", &self.throttled_until)
        .field("errored", &self.errored)
        .field("invocation_id", &self.invocation_id)
        .finish()
    }
}

impl Notifier {
    pub(crate) fn new(invocation_id: uuid::Uuid, source: Arc<dyn TaskSource>,) -> Notifier {
        Notifier {
            score: None,
            test: None,
            throttled_until: Instant::now(),
            errored: false,
            source,
            invocation_id
        }
    }

    pub(crate) async fn set_score(&mut self, score: u32) {
        self.score = Some(score);
        self.maybe_drain().await
    }

    pub(crate) async fn set_test(&mut self, test: u32) {
        self.test = Some(test);
        self.maybe_drain().await
    }

    async fn maybe_drain(&mut self) {
        let mut has_something = false;
        has_something = has_something || self.score.is_some();
        has_something = has_something || self.test.is_some();
        if !has_something {
            return;
        }
        if self.errored {
            return;
        }
        if self.throttled_until > Instant::now() {
            return;
        }
        self.drain().await
    }

    async fn drain(&mut self) {
        debug!("Notifier: draining");
        let event = invoker_api::LiveStatusUpdate {
            score: self.score.take().map(|x| x as i32),
            current_test: self.test.take(),
        };
        debug!("Sending live status update for invocation {}", self.invocation_id);
        if let Err(err) = self.source.deliver_live_status_update(self.invocation_id, event).await {
            warn!("Failed to send live status update: {}", err);
            warn!("Disabling live status updates for this run");
            self.errored = true;
        }
        self.throttled_until = Instant::now() + LIVE_STATUS_UPDATE_THROTTLE;
    }
}

const LIVE_STATUS_UPDATE_THROTTLE: Duration = Duration::from_millis(250);
