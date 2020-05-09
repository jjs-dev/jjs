mod from_request;

use super::{
    security::{Action, Authorizer, Operation, Outcome, ResourceKind, Token, TokenMgr},
    ApiError, ApiResult, ResultToApiUtil as _,
};
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct DbContext {
    db_connection: db::DbConn,
}

impl DbContext {
    pub(crate) fn db(&self) -> &db::DbConn {
        &self.db_connection
    }

    pub(crate) fn create(db_connection: db::DbConn) -> DbContext {
        DbContext { db_connection }
    }
}

pub(crate) struct TokenManageContext {
    mgr: TokenMgr,
}

impl TokenManageContext {
    pub(crate) fn token_manager(&self) -> &TokenMgr {
        &self.mgr
    }
}

pub(crate) struct CredentialsContext {
    token: Rc<Token>,
}

impl CredentialsContext {
    pub(crate) fn token(&self) -> &Rc<Token> {
        &self.token
    }
}

#[derive(Clone)]
pub(crate) struct EntityContext {
    entity_loader: entity::Loader,
    problem_loader: problem_loader::Loader,
}

impl EntityContext {
    pub(crate) fn entities(&self) -> &entity::Loader {
        &self.entity_loader
    }

    pub(crate) fn problems(&self) -> &problem_loader::Loader {
        &self.problem_loader
    }

    pub(crate) fn create(
        entity_loader: entity::Loader,
        problem_loader: problem_loader::Loader,
    ) -> EntityContext {
        EntityContext {
            entity_loader,
            problem_loader,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ConfigContext {
    apiserver_config: Rc<crate::config::ApiserverConfig>,
    data_dir: Rc<std::path::Path>,
}

impl ConfigContext {
    pub(crate) fn config(&self) -> &Rc<crate::config::ApiserverConfig> {
        &self.apiserver_config
    }

    pub(crate) fn data_dir(&self) -> &Rc<std::path::Path> {
        &self.data_dir
    }
}

pub(crate) struct SecurityContext {
    authorizer: Authorizer,
    cred_cx: CredentialsContext,
}

impl SecurityContext {
    pub(crate) fn access(&self) -> SecutityOperationBuilder<TagFalse, TagFalse, TagFalse> {
        let dy = DynSecutityOperationBuilder {
            action: None,
            conditions: None,
            resource_kind: None,
            user_info: self.cred_cx.token().user_info.clone(),
            authorizer: self.authorizer.clone(),
        };
        SecutityOperationBuilder(dy, std::marker::PhantomData)
    }
}

#[must_use]
pub(crate) struct DynSecutityOperationBuilder {
    action: Option<Action>,
    conditions: Option<anymap::AnyMap>,
    resource_kind: Option<ResourceKind>,
    user_info: super::security::UserInfo,
    authorizer: Authorizer,
}

impl DynSecutityOperationBuilder {
    fn into_operation(self) -> Operation {
        Operation {
            action: self.action.unwrap(),
            conditions: Rc::new(self.conditions.unwrap()),
            resource_kind: self.resource_kind.unwrap(),
            user_info: self.user_info,
        }
    }

    pub(crate) async fn try_authorize(self) -> ApiResult<Outcome> {
        let authorizer = self.authorizer.clone();
        let op = self.into_operation();
        authorizer.authorize(&op).await.internal()
    }

    pub(crate) async fn authorize(self) -> ApiResult<()> {
        let outcome = self.try_authorize().await?;
        match outcome.deny_message() {
            None => Ok(()),
            Some(msg) => {
                let mut err = ApiError::access_denied();
                err.cause = Some(anyhow::anyhow!("Operation denied: {}", msg));
                Err(err)
            }
        }
    }

    pub(crate) fn with_action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self
    }

    pub(crate) fn with_resource_kind(mut self, rk: ResourceKind) -> Self {
        self.resource_kind = Some(rk);
        self
    }

    pub(crate) fn with_conditions(mut self, conds: anymap::AnyMap) -> Self {
        self.conditions = Some(conds);
        self
    }
}

pub(crate) enum TagTrue {}

pub(crate) enum TagFalse {}

#[must_use]
pub(crate) struct SecutityOperationBuilder<A, C, RK>(
    DynSecutityOperationBuilder,
    std::marker::PhantomData<(A, C, RK)>,
);

impl SecutityOperationBuilder<TagTrue, TagTrue, TagTrue> {
    pub(crate) async fn try_authorize(self) -> ApiResult<Outcome> {
        self.0.try_authorize().await
    }

    pub(crate) async fn authorize(self) -> ApiResult<()> {
        self.0.authorize().await
    }
}

impl<C, RK> SecutityOperationBuilder<TagFalse, C, RK> {
    pub(crate) fn with_action(self, action: Action) -> SecutityOperationBuilder<TagTrue, C, RK> {
        SecutityOperationBuilder(self.0.with_action(action), std::marker::PhantomData)
    }
}

impl<A, RK> SecutityOperationBuilder<A, TagFalse, RK> {
    pub(crate) fn with_conditions(
        self,
        conds: anymap::AnyMap,
    ) -> SecutityOperationBuilder<A, TagTrue, RK> {
        SecutityOperationBuilder(self.0.with_conditions(conds), std::marker::PhantomData)
    }
}

impl<A, C> SecutityOperationBuilder<A, C, TagFalse> {
    pub(crate) fn with_resource_kind(
        self,
        rk: ResourceKind,
    ) -> SecutityOperationBuilder<A, C, TagTrue> {
        SecutityOperationBuilder(self.0.with_resource_kind(rk), std::marker::PhantomData)
    }
}
