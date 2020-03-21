use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "gram.pest"]
struct RawAstParser;

pub(crate) type ParseError = pest::error::Error<Rule>;

pub(crate) enum Option {
    Flag(String),
    Setting(String, String),
}

pub(crate) enum StatementData {
    AddUser {
        username: String,
        password: String,
        options: Vec<Option>,
    },
    SetOpt {
        options: Vec<Option>,
    },
}

pub struct Statement {
    pub(crate) data: StatementData,
}

fn parse_options(ast: Pair<Rule>) -> Vec<Option> {
    let mut out = vec![];
    for option_ast in ast.into_inner() {
        let child = option_ast.into_inner().next().unwrap();
        let option = match child.as_rule() {
            Rule::setting => {
                let mut iter = child.into_inner();
                let name = iter.next().unwrap().as_str().to_string();
                let val = iter.next().unwrap().as_str().to_string();
                Option::Setting(name, val)
            }
            Rule::flag => {
                let name = child.as_str().to_string();
                Option::Flag(name)
            }
            _ => unimplemented!(),
        };
        out.push(option);
    }
    out
}

pub(crate) fn parse(data: &str) -> Result<Vec<Statement>, crate::Error> {
    let ast = RawAstParser::parse(Rule::userlist, data)?.next().unwrap();
    let mut out = vec![];
    for item in ast.into_inner() {
        if item.as_rule() == Rule::EOI {
            break;
        }
        let item = item.into_inner().next().unwrap();
        let sdata = match item.as_rule() {
            Rule::statement_adduser => {
                let mut iter = item.into_inner();
                let username = iter.next().unwrap().as_str().to_string();
                let password = iter.next().unwrap().as_str().to_string();
                let options = iter.next().map(parse_options).unwrap_or_else(Vec::new);
                StatementData::AddUser {
                    username,
                    password,
                    options,
                }
            }
            Rule::statement_setopts => {
                let options_ast = item.into_inner().next().unwrap();
                let options = parse_options(options_ast);
                StatementData::SetOpt { options }
            }
            _ => unreachable!(),
        };
        let st = Statement { data: sdata };
        out.push(st);
    }
    Ok(out)
}
