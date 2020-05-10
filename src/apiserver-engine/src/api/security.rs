//mod access_ck;
pub mod rules;
mod token;
mod token_mgr;

pub use token::{Token, UserInfo};
pub use token_mgr::{TokenMgr, TokenMgrError};
pub mod resource_ident;

use anyhow::Context as _;
use futures::future::FutureExt;
use log::debug;
use std::{borrow::Cow, rc::Rc};

// atomic authentication unit: either Allowed or Denied.
#[derive(Debug)]
pub struct Operation {
    pub user_info: UserInfo,
    pub resource_kind: ResourceKind,
    pub action: Action,
    pub conditions: anymap::AnyMap,
}

impl Operation {
    pub fn get_condition<T: 'static>(&self) -> &T {
        self.conditions
            .get()
            .unwrap_or_else(|| panic!("{} condition missing", std::any::type_name::<T>()))
    }
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
    /// Represents contest per-user settings (e.g. querying/changing participation status)
    pub const CONTEST_PARTICIPATION: Self = ResourceKind(Cow::Borrowed("/contest/participation"));
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
    fn authorize_operation(&self, op: &Rc<Operation>) -> RuleRet;

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

/// Conjunction of several `Rule`s
pub struct Pipeline {
    name: Rc<str>,
    rules: Rc<[Box<dyn Rule>]>,
}

pub struct PipelineBuilder {
    name: String,
    rules: Vec<Box<dyn Rule>>,
}

impl Pipeline {
    pub fn builder() -> PipelineBuilder {
        PipelineBuilder {
            rules: (vec![]),
            name: "unnamed".to_string(),
        }
    }

    /// Authorizes operation.
    /// # Flow
    /// - If any rule returns Deny, return this explicit `deny`.
    /// - If all rule had no opinion, return implicit `deny`.
    /// - Otherwise, return `allow`.
    pub async fn authorize(&self, op: &Rc<Operation>) -> anyhow::Result<Outcome> {
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
                reason: "implicit deny: no rule approved requsted operation".to_string(),
            })
        }
    }
}

impl PipelineBuilder {
    pub fn add_rule(&mut self, rule: Box<dyn Rule>) -> &mut Self {
        self.rules.push(rule);
        self
    }

    pub fn set_name(&mut self, name: String) -> &mut Self {
        self.name = name;
        self
    }

    pub fn build(self) -> Pipeline {
        Pipeline {
            rules: self.rules.into(),
            name: self.name.into(),
        }
    }
}

/// Disjunction of several `Pipeline`s
#[derive(Clone)]
pub struct Authorizer {
    pipelines: Rc<[Pipeline]>,
}

impl Authorizer {
    pub fn builder() -> AuthorizerBuilder {
        AuthorizerBuilder {
            pipelines: (Vec::new()),
        }
    }

    /// Authorizes operation
    /// # Flow
    /// - If any pipeline returns Allow, return this Allow
    /// - Otherwise, return Deny
    pub async fn authorize(&self, operation: &Rc<Operation>) -> anyhow::Result<Outcome> {
        assert!(!self.pipelines.is_empty());
        debug!("{:?}", operation);
        let mut denies = Vec::new();
        for pipeline in &*self.pipelines {
            let outcome = pipeline
                .authorize(operation)
                .await
                .with_context(|| format!("Pipeline {} failed", pipeline.name))?;
            match outcome {
                Outcome::Allow => return Ok(Outcome::Allow),
                Outcome::Deny { reason } => denies.push(reason),
            }
        }
        Ok(Outcome::Deny {
            reason: std::convert::identity(denies).remove(0),
        })
    }
}

pub struct AuthorizerBuilder {
    pipelines: Vec<Pipeline>,
}

impl AuthorizerBuilder {
    pub fn add_pipeline(&mut self, pipeline: Pipeline) -> &mut Self {
        self.pipelines.push(pipeline);
        self
    }

    pub fn build(self) -> Authorizer {
        Authorizer {
            pipelines: self.pipelines.into(),
        }
    }
}

pub fn create_sudo_pipeline() -> Pipeline {
    let mut builder = Pipeline::builder();
    builder.add_rule(Box::new(SudoRule));
    builder.build()
}

pub struct SudoRule;

impl Rule for SudoRule {
    fn name(&self) -> &'static str {
        "Sudo"
    }

    fn description(&self) -> &'static str {
        "Authorizes all requests made by superuser"
    }

    fn authorize_operation(&self, op: &Rc<Operation>) -> RuleRet {
        if op.user_info.name != "Global/Root" {
            return futures::future::ok(None).boxed();
        }
        debug!("SudoRule: Approving operation {:?}", op);
        futures::future::ok(Some(Outcome::Allow)).boxed()
    }
}
