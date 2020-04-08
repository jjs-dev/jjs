#![feature(backtrace)]
// for async-trait
#![allow(clippy::needless_lifetimes)]
pub mod config;
pub mod data;
pub mod db;
pub mod problems;
pub mod toolchains;

use async_trait::async_trait;

#[async_trait]
pub trait Component: std::fmt::Display {
    type Error: std::error::Error + Send + Sync;
    async fn state(&self) -> Result<StateKind, Self::Error>;
    async fn upgrade(&self) -> Result<(), Self::Error>;
    fn name(&self) -> &'static str;
}

#[derive(Debug)]
pub enum StateKind {
    UpToDate,
    Upgradable,
    Errored,
}

impl std::fmt::Display for StateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
