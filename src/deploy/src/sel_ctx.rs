use crate::Params;

/// Selection context
pub struct SelCtx<'sctx> {
    params: &'sctx Params,
}

impl<'sctx> SelCtx<'sctx> {
    pub(crate) fn components_cfg(&self) -> &crate::cfg::ComponentsConfig {
        &self.params.cfg.components
    }

    pub(crate) fn new(params: &'sctx Params) -> Self {
        Self { params }
    }
}
