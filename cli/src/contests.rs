use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Opt {
    contest: Option<String>,
}

pub fn exec(opt: Opt, common: &super::CommonParams) {
    match opt.contest {
        Some(name) => {
            let info = common
                .client
                .contests_describe(&name)
                .expect("network error")
                .expect("error");
            println!("contest name: {}", &info.name);
            println!("contest title: {}", &info.title);
        }
        None => {
            let information = common
                .client
                .contests_list(&())
                .expect("network error")
                .expect("error");
            for (i, contest) in information.iter().enumerate() {
                println!("{}) {}", i + 1, contest.name);
            }
        }
    };
}
