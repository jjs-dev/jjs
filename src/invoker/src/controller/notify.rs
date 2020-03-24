//! Implements Notifications - messages about run testing updates
use log::{debug, warn};
use std::time::{Duration, Instant};
#[derive(Debug)]
pub(crate) struct Notifier {
    score: Option<u32>,
    test: Option<u32>,
    endpoint: Option<String>,
    throttled_until: Instant,
    errored: bool,
}

impl Notifier {
    pub(crate) fn new(endpoint: Option<String>) -> Notifier {
        Notifier {
            score: None,
            test: None,
            endpoint,
            throttled_until: Instant::now(),
            errored: false,
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
        let endpoint = match self.endpoint.as_ref() {
            Some(ep) => ep,
            None => return,
        };
        let event = invoker_api::LiveStatusUpdate {
            score: self.score.take().map(|x| x as i32),
            current_test: self.test.take(),
        };
        let client = reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .expect("failed to initialize reqwest client");
        debug!("Sending request to {}", &endpoint);
        if let Err(err) = client.post(endpoint).json(&event).send().await {
            warn!("Failed to send live status update: {}", err);
            warn!("Disabling live status updates for this run");
            self.errored = true;
        }
        self.throttled_until = Instant::now() + LIVE_STATUS_UPDATE_THROTTLE;
    }
}

const LIVE_STATUS_UPDATE_THROTTLE: Duration = Duration::from_secs(1);
