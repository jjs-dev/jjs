use acl::{
    self, Effect, Entry, Item, Object, Prefix, RuleSubject, SecurityDescriptor,
    SPECIAL_SEGMENT_SUDO,
};
use bitflags::bitflags;

bitflags! {
    pub struct ContestRights: u64 {
        /// Submit solution, when contest is running
        const SUBMIT = 1;

        /// Judge mode TODO: split
        const JUDGE = 1 << 1;

        /// View contest
        const VIEW = 1 << 2;
    }
}

bitflags! {
    pub struct GlobalRights: u64 {
        /// Manage users
        const MANAGE_USERS = 1;
    }
}

const LOGGED_IN_GROUP: &str = "$JJS.LoggedIn";

#[derive(Debug)]
pub struct AccessControlData {
    pub root: Prefix,
}

pub fn init_contest(cfg: &cfg::Contest) -> Prefix {
    let mut root = Prefix::new(); // no global restrictions

    let contest_name = "TODO";
    {
        let sudoers_entry = Entry {
            subject: RuleSubject::Group(format!("Contest-{}-Sudoers", contest_name)),
            effect: Effect::Allow(None),
        };

        root.add_item(
            SPECIAL_SEGMENT_SUDO,
            Item::Object(Object {
                security: SecurityDescriptor {
                    acl: vec![sudoers_entry],
                },
            }),
        );
    }
    {
        let rights_participants = ContestRights::SUBMIT | ContestRights::VIEW;
        let contest_common_rights_participants = Entry {
            subject: RuleSubject::Group(cfg.group.clone()),
            effect: Effect::Allow(Some(rights_participants.bits())),
        };

        let rights_judges = rights_participants | ContestRights::JUDGE;

        let contest_common_rights_judges = Entry {
            subject: RuleSubject::Group("Judges".to_string()),
            effect: Effect::Allow(Some(rights_judges.bits())),
        };

        let mut acl = vec![
            contest_common_rights_participants,
            contest_common_rights_judges,
        ];
        let spectator_rights = ContestRights::VIEW;

        if cfg.anon_visible {
            let entry = Entry {
                subject: RuleSubject::Everyone,
                effect: Effect::Allow(Some(spectator_rights.bits())),
            };
            acl.push(entry);
        }

        if cfg.unregistered_visible {
            let entry = Entry {
                subject: RuleSubject::Group(LOGGED_IN_GROUP.to_string()),
                effect: Effect::Allow(Some(spectator_rights.bits())),
            };
            acl.push(entry);
        }

        let common_rights_obj_name = "CommonRights";

        root.add_item(
            common_rights_obj_name,
            Item::Object(Object {
                security: SecurityDescriptor { acl },
            }),
        );
    }
    root
}

pub fn init(cfg: &cfg::Config) -> AccessControlData {
    let mut root = Prefix::new();
    root.add_item("Contest", Item::Prefix(init_contest(&cfg.contests[0])));
    {
        let sudoers_acl = vec![Entry {
            subject: RuleSubject::Group("Sudoers".to_string()),
            effect: Effect::Allow(None),
        }];

        root.add_item(
            SPECIAL_SEGMENT_SUDO,
            Item::Object(Object {
                security: SecurityDescriptor { acl: sudoers_acl },
            }),
        );
    }
    {
        let common_rights_acl = vec![];

        root.add_item(
            "CommonRights",
            Item::Object(Object {
                security: SecurityDescriptor {
                    acl: common_rights_acl,
                },
            }),
        );
    }

    AccessControlData { root }
}
