//mod access_ck;
pub mod rules;
mod token;
mod token_mgr;

pub use token::{Token, UserInfo};
pub use token_mgr::{TokenMgr, TokenMgrError};
pub mod resource_ident;

use anyhow::Context as _;
use std::{borrow::Cow, rc::Rc};

// atomic authentication unit: either Allowed or Denied.
#[derive(Clone, Debug)]
pub struct Operation {
    pub user_info: UserInfo,
    pub resource_kind: ResourceKind,
    pub action: Action,
    pub conditions: Rc<anymap::AnyMap>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Action {
    Create,
    Delete,
    Get,
    Patch,
    List,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ResourceKind(Cow<'static, str>);

impl ResourceKind {
    /// Ident: contest name.
    pub const CONTEST: Self = ResourceKind(Cow::Borrowed("/contest"));
    /// Ident: contest name.
    pub const RUN: Self = ResourceKind(Cow::Borrowed("/run"));
    /// Represents all runs
    pub const RUNS_LIST: Self = ResourceKind(Cow::Borrowed("/runs"));
    /// Ident: contest name.
    pub const RUN_PROTOCOL: Self = ResourceKind(Cow::Borrowed("/run/protocol"));
    /// Resource for arbitrary users modifications.
    /// No conditions are provided.
    pub const USERS: Self = ResourceKind(Cow::Borrowed("/users"));
    /// Represents all runs by some user
    pub const USER_RUNS_LIST: Self = ResourceKind(Cow::Borrowed("/users/runs"));
}
type RuleRet =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<Outcome>, anyhow::Error>>>>;
pub trait Rule {
    /// Err(_) - request aborted, internal error.
    /// Ok(None) - no opinion.
    /// Ok(Allow) - allow request.
    /// Ok(Deny) - explicit deny.
    fn authorize_operation(&self, op: &Operation) -> RuleRet;

    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;
}

pub enum Outcome {
    Allow,
    Deny { reason: String },
}

impl Outcome {
    pub fn deny_message(&self) -> Option<&str> {
        match self {
            Outcome::Allow => None,
            Outcome::Deny { reason } => Some(reason),
        }
    }

    pub fn is_allow(&self) -> bool {
        matches!(self, Outcome::Allow)
    }
}

#[derive(Clone)]
pub struct Authorizer {
    rules: Rc<Vec<Box<dyn Rule>>>,
}

impl Authorizer {
    pub fn builder() -> AuthorizerBuilder {
        AuthorizerBuilder(Authorizer {
            rules: Rc::new(Vec::new()),
        })
    }

    /// Authorizes operation.
    /// # Flow
    /// - If any rule returns Deny, return this explicit `deny`.
    /// - If all rule had no opinion, return implicit `deny`.
    /// - Otherwise, return `allow`.
    pub async fn authorize(&self, op: Operation) -> anyhow::Result<Outcome> {
        let mut had_allow = false;
        for rule in self.rules.iter() {
            let outcome = rule
                .authorize_operation(&op)
                .await
                .with_context(|| format!("Rule {} had internal error", rule.name()))?;
            match outcome {
                Some(Outcome::Allow) => had_allow = true,
                Some(deny @ Outcome::Deny { .. }) => return Ok(deny),
                None => (),
            }
        }
        if had_allow {
            Ok(Outcome::Allow)
        } else {
            Ok(Outcome::Deny {
                reason: "implicit deny: no rule approved this".to_string(),
            })
        }
    }
}

pub struct AuthorizerBuilder(Authorizer);

impl AuthorizerBuilder {
    pub fn add_rule(&mut self, rule: Box<dyn Rule>) -> &mut Self {
        // Arc::get_mut returns Some, because `AuthorizerBuilder` never
        // clones, so RC is unique.
        Rc::get_mut(&mut self.0.rules)
            .expect("AuthorizerBuilder does not allow cloning it's Rc")
            .push(rule);
        self
    }

    pub fn build(self) -> Authorizer {
        self.0
    }
}
