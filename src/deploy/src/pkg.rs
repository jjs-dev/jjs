//! Abstractions for package
use crate::{build_ctx::BuildCtx, inst_ctx::InstallCtx, sel_ctx::SelCtx};

pub(crate) trait Package: std::fmt::Debug {
    fn check_selected(&mut self, sctx: &SelCtx);

    fn selected(&self) -> bool;

    fn build(&self, bctx: &BuildCtx);

    fn install(&self, ictx: &InstallCtx);
}

#[derive(Debug)]
pub(crate) enum PackageComponentKind {
    Core,
    Tools,
    Extra,
}
