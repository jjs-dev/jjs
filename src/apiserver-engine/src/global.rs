use invoker_api::LiveStatusUpdate;
use slog_scope::{error, warn};
use std::{
    collections::{HashMap, VecDeque},
    io::{Read, Write},
    time::{Duration, Instant},
};

/// Defines some information, not bound to concrete request
pub struct GlobalState {
    pub(crate) live_status_updates: LiveStatusUpdatesCache,
}

impl GlobalState {
    pub fn new() -> GlobalState {
        GlobalState {
            live_status_updates: LiveStatusUpdatesCache::new(),
        }
    }
}

impl Default for GlobalState {
    fn default() -> Self {
        Self::new()
    }
}

const LSU_KEY_BYTE_LENGTH: usize = 20;

#[derive(Debug, Hash, Copy, Clone, Eq, PartialEq)]
pub(crate) struct LsuKey {
    /// Use who submitted run
    pub(crate) user: uuid::Uuid,
    /// Run ID
    pub(crate) run: u32,
}

impl LsuKey {
    fn write(&self, mut wr: impl Write) -> std::io::Result<()> {
        wr.write_all(self.user.as_bytes())?;
        wr.write_all(&self.run.to_ne_bytes())
    }

    fn read(mut rd: impl Read) -> std::io::Result<Self> {
        let mut buf_user = [0; 16];
        let mut buf_run_id = [0; 4];
        rd.read_exact(&mut buf_user)?;
        rd.read_exact(&mut buf_run_id)?;
        Ok(LsuKey {
            user: uuid::Uuid::from_bytes(buf_user),
            run: u32::from_ne_bytes(buf_run_id),
        })
    }
}

/// How long status update will stay in cache.
const LSU_LIFE_TIME: Duration = Duration::from_secs(3);

pub(crate) struct LiveStatusUpdatesCache {
    /// Stores last LSU associated with run
    cache: HashMap<LsuKey, LiveStatusUpdate>,
    /// Used to delete old entries. First item is key, second is time when key can be deleter
    deletion_queue: VecDeque<(LsuKey, Instant)>,
    /// Used to sign URLs
    signing_key: [u8; 32],
}

impl LiveStatusUpdatesCache {
    pub fn make_token(&self, key: LsuKey) -> String {
        let mut buf = [0u8; LSU_KEY_BYTE_LENGTH];
        key.write(&mut buf[..]).unwrap();
        let buf = base64::encode(&buf);
        branca::encode(&buf, &self.signing_key, 0).unwrap()
    }

    pub fn webhook_handler(&mut self, lsu: LiveStatusUpdate, token: String) {
        let token = match branca::decode(&token, &self.signing_key, 0) {
            Ok(tok) => tok,
            Err(err) => {
                warn!(
                    "discarding lsu webhook request: token signature is invalid: {}",
                    err
                );
                return;
            }
        };
        let key = match base64::decode(&token)
            .map_err(|branca_err| std::io::Error::new(std::io::ErrorKind::Other, branca_err))
            .and_then(|buf| LsuKey::read(buf.as_slice()))
        {
            Ok(key) => key,
            Err(err) => {
                error!(
                    "LSU webhook: token signature is valid, but token payload is invalid: {}",
                    err
                );
                return;
            }
        };

        self.push(key, lsu)
    }

    fn new() -> LiveStatusUpdatesCache {
        LiveStatusUpdatesCache {
            cache: HashMap::new(),
            deletion_queue: VecDeque::new(),
            signing_key: rand::random(),
        }
    }

    fn delete_old(&mut self) {
        let now = Instant::now();
        while let Some(oldest) = self.deletion_queue.front() {
            if oldest.1 < now {
                let key = oldest.0;
                self.cache.remove(&key);
                self.deletion_queue.pop_front();
            } else {
                break;
            }
        }
    }

    fn push(&mut self, key: LsuKey, lsu: LiveStatusUpdate) {
        self.delete_old();
        let now = Instant::now();
        self.deletion_queue.push_back((key, now + LSU_LIFE_TIME));
        self.cache.insert(key, lsu);
    }

    pub(crate) fn extract(&mut self, key: LsuKey) -> Option<LiveStatusUpdate> {
        self.cache.remove(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsu_key_byte_length_is_correct() {
        let mut buf = [0; LSU_KEY_BYTE_LENGTH];
        let lsu_key = LsuKey {
            user: Default::default(),
            run: 0,
        };
        let mut slice = &mut buf[..];
        lsu_key.write(&mut slice).unwrap();
        assert!(slice.is_empty())
    }

    #[test]
    fn test_roundtrip() {
        let mut buf = Vec::new();
        let lsu_key = LsuKey {
            run: 0x1234_5678,
            user: uuid::Uuid::from_bytes(
                0x1001_3223_5445_7667_9889_baab_dccd_feefu128.to_ne_bytes(),
            ),
        };
        lsu_key.write(&mut buf).unwrap();
        let lsu_key2 = LsuKey::read(&mut buf.as_slice()).unwrap();
        assert_eq!(lsu_key, lsu_key2)
    }
}
