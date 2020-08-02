#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BodyRoutePutProblemProblemsProblemIdPut {
    pub problem_assets: String,
    pub problem_manifest: String,
}

impl BodyRoutePutProblemProblemsProblemIdPut {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> BodyRoutePutProblemProblemsProblemIdPutBuilder<crate::generics::MissingProblemAssets, crate::generics::MissingProblemManifest> {
        BodyRoutePutProblemProblemsProblemIdPutBuilder {
            body: Default::default(),
            _problem_assets: core::marker::PhantomData,
            _problem_manifest: core::marker::PhantomData,
        }
    }
}

impl Into<BodyRoutePutProblemProblemsProblemIdPut> for BodyRoutePutProblemProblemsProblemIdPutBuilder<crate::generics::ProblemAssetsExists, crate::generics::ProblemManifestExists> {
    fn into(self) -> BodyRoutePutProblemProblemsProblemIdPut {
        self.body
    }
}

/// Builder for [`BodyRoutePutProblemProblemsProblemIdPut`](./struct.BodyRoutePutProblemProblemsProblemIdPut.html) object.
#[derive(Debug, Clone)]
pub struct BodyRoutePutProblemProblemsProblemIdPutBuilder<ProblemAssets, ProblemManifest> {
    body: self::BodyRoutePutProblemProblemsProblemIdPut,
    _problem_assets: core::marker::PhantomData<ProblemAssets>,
    _problem_manifest: core::marker::PhantomData<ProblemManifest>,
}

impl<ProblemAssets, ProblemManifest> BodyRoutePutProblemProblemsProblemIdPutBuilder<ProblemAssets, ProblemManifest> {
    #[inline]
    pub fn problem_assets(mut self, value: impl Into<String>) -> BodyRoutePutProblemProblemsProblemIdPutBuilder<crate::generics::ProblemAssetsExists, ProblemManifest> {
        self.body.problem_assets = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn problem_manifest(mut self, value: impl Into<String>) -> BodyRoutePutProblemProblemsProblemIdPutBuilder<ProblemAssets, crate::generics::ProblemManifestExists> {
        self.body.problem_manifest = value.into();
        unsafe { std::mem::transmute(self) }
    }
}
