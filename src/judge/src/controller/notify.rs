//! Implements Notifications - messages about run testing updates
use crate::controller::JudgeResponseCallbacks;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{debug, instrument, warn};
pub(crate) struct Notifier {
    score: Option<u32>,
    test: Option<u32>,
    throttled_until: Instant,
    errored: bool,
    handler: Arc<dyn JudgeResponseCallbacks>,
    judge_request_id: uuid::Uuid,
}

impl std::fmt::Debug for Notifier {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Notifier")
            .field("score", &self.score)
            .field("test", &self.test)
            .field("throttled_until", &self.throttled_until)
            .field("errored", &self.errored)
            .field("judge_request_id", &self.judge_request_id)
            .finish()
    }
}

impl Notifier {
    pub(crate) fn new(
        judge_request_id: uuid::Uuid,
        handler: Arc<dyn JudgeResponseCallbacks>,
    ) -> Notifier {
        Notifier {
            score: None,
            test: None,
            throttled_until: Instant::now(),
            errored: false,
            handler,
            judge_request_id,
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

    #[instrument(skip(self), fields(judge_request_id=%self.judge_request_id))]
    async fn drain(&mut self) {
        let event = judging_apis::LiveStatusUpdate {
            score: self.score.take().map(|x| x as i32),
            current_test: self.test.take(),
        };
        debug!(
            update=?event,
            "Sending live status update",
        );
        if let Err(err) = self
            .handler
            .deliver_live_status_update(self.judge_request_id, event)
            .await
        {
            warn!(error=%format_args!("{:#}", err), "Failed to send live status update");
            warn!("Disabling live status updates for this run");
            self.errored = true;
        }
        self.throttled_until = Instant::now() + LIVE_STATUS_UPDATE_THROTTLE;
    }
}

const LIVE_STATUS_UPDATE_THROTTLE: Duration = Duration::from_millis(250);
