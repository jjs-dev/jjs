mod step;
mod fixture;

use std::{env, fs, io::BufReader, ops::Not, process::exit};

use step::Step;


struct TestCase {
    name: String,
    steps: Vec<Step>,
    fixture_params: fixture::FixtureParams
}

impl TestCase {
    fn parse(v: &serde_yaml::Value) -> TestCase {
        let steps = v.get("steps").unwrap().as_sequence().unwrap();
        let steps = steps.iter().map(|v| Step::parse(v)).collect::<Vec<_>>();
        let fixture_params = v.get("env").map(  fixture::FixtureParams::parse).unwrap_or_default();
        TestCase {
            name: "".to_string(),
            steps,
            fixture_params
        }
    }
}



fn check_snapshot(test_case: TestCase) -> bool {
    println!("running snapshot {}", &test_case.name);
    let server = test_case.fixture_params.into_app(&test_case.name);
    let client = rocket::local::Client::new(server).unwrap();
    for step in test_case.steps {
        if !step.run(&client) {
            return false;
        }
    }
    true
}

pub fn main() {
    println!("running snapshot-based tests");
    let snapshots_dir = env::current_dir().unwrap().join("tests/snapshots");
    let items = fs::read_dir(&snapshots_dir).unwrap();
    let mut errors = Vec::new();
    for item in items {
        let item = item.unwrap();
        let path = item.path();
        let test_case_data = fs::File::open(&path).unwrap();
        let test_case_data = BufReader::new(test_case_data);
        let test_case_data = serde_yaml::from_reader(test_case_data).unwrap();
        let mut test_case = TestCase::parse(&test_case_data);
        test_case.name = path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .split('.')
            .next()
            .unwrap()
            .to_string();
        let name = test_case.name.clone();
        if !check_snapshot(test_case) {
            errors.push(name);
        }
    }
    if errors.is_empty().not() {
        eprintln!("Error: some snapshots failed: {:?}", &errors);
        exit(1);
    }
}