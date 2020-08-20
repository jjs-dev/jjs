use std::time::{Duration, Instant};

const STEP_PERCENTAGE_THRESHOLD: usize = 20;
const STEP_DURATION_THRESHOLD: Duration = Duration::from_secs(10);

pub(super) struct Notifier {
    last_step: usize,
    total_step_count: usize,
    last_time: std::time::Instant,
}

impl Notifier {
    pub(super) fn new(cnt: usize) -> Notifier {
        assert_ne!(cnt, 0);
        Notifier {
            last_step: 0,
            total_step_count: cnt,
            last_time: Instant::now(),
        }
    }

    fn do_notify(&mut self, new_step: usize) {
        println!("Progress: {}/{}", new_step, self.total_step_count);
        self.last_step = new_step;
        self.last_time = Instant::now();
    }

    pub(super) fn maybe_notify(&mut self, new_step: usize) {
        let mut should_notify = false;
        {
            let cnt_delta = new_step - self.last_step;
            if 100 * cnt_delta >= STEP_PERCENTAGE_THRESHOLD * self.total_step_count {
                should_notify = true;
            }
        }
        {
            let time_delta = Instant::now().duration_since(self.last_time);
            if time_delta >= STEP_DURATION_THRESHOLD {
                should_notify = true;
            }
        }
        if should_notify {
            self.do_notify(new_step);
        }
    }
}
