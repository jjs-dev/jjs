use std::{
    io::{self, Read},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct CancelToken {
    closed: AtomicBool,
}

impl CancelToken {
    pub fn cancel(&self) {
        self.closed.store(true, Ordering::SeqCst)
    }

    pub fn new() -> CancelToken {
        CancelToken {
            closed: AtomicBool::new(false),
        }
    }
}

pub struct CancellableReader<'a, R: Read> {
    reader: &'a mut R,
    token: &'a CancelToken,
}

impl<'a, R:Read> CancellableReader<'a, R> {
    fn is_closed(&self) -> bool {
        self.token.closed.load(Ordering::SeqCst)
    }
}

impl<'a, R: Read> Read for CancellableReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        if self.is_closed(){
            return Ok(0)
        }
        let res = self.reader.read(buf);
        if self.is_closed() {
            return Ok(0)
        }
        res
    }
}

impl<'a, R: Read> CancellableReader<'a, R> {
    pub fn new(r: &'a mut R, token: &'a CancelToken) -> Self {
        Self {
            reader: r,
            token: &token,
        }
    }
}
