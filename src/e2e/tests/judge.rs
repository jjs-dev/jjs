use std::convert::TryInto;

fn participate() {
    e2e::RequestBuilder::new()
        .method(apiserver_engine::test_util::Method::Patch)
        .action("/contests/trial/participation")
        .var("phase", "ACTIVE")
        .exec();
}

fn submit(code: &str) -> i32 {
    let code = base64::encode(code);

    let resp = e2e::RequestBuilder::new()
        .action("/runs")
        .var("toolchain", "g++")
        .var("problem", "A")
        .var("contest", "trial")
        .var("code", code.as_str())
        .exec()
        .unwrap_ok();
    let resp = resp.pointer("/id").unwrap();
    resp.as_i64().unwrap().try_into().unwrap()
}

fn poll_status(id: i32) -> String {
    let run_info = e2e::RequestBuilder::new()
        .action(&format!("/runs/{}", id))
        .exec()
        .unwrap_ok();

    let status = &run_info["status"];
    status
        .as_object()
        .map(|s| s.get("code").unwrap().as_str().unwrap().to_string())
        .unwrap_or_else(|| "QUEUE".to_string())
}

fn send_check_status(run_code: &str, correct_status: &str) {
    let id = submit(run_code);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
    loop {
        if std::time::Instant::now() > deadline {
            panic!("Timeout");
        }
        let status = poll_status(id);
        if status == "QUEUE" {
            continue;
        }
        if status == correct_status {
            break;
        } else {
            panic!("Unexpected status: {}, expected {}", status, correct_status);
        }
    }
}

#[test]
fn test_correct_solution_is_accepted() {
    participate();
    send_check_status(
        r#"
 #include <cstdio>
 int main() {
     long long a, b;
     scanf("%lld %lld", &a, &b);
     printf("%lld\n", a+b);
 }    
     "#,
        "ACCEPTED",
    );
}

#[test]
fn test_wrong_solution_is_rejected() {
    participate();
    send_check_status(
        r#"
 #include <cstdio>
 int main() {
     long long a,b;
     scanf("%lld %lld", &a, &b);
     printf("%lld\n", a-b);
 }       
        "#,
        "PARTIAL_SOLUTION",
    )
}

#[test]
fn test_non_privileged_user_cannot_see_non_their_runs() {
    participate();
    e2e::RequestBuilder::new()
        .action("/users")
        .var("login", "cersei")
        .var("groups", &[] as &[String])
        .var("password", "")
        .exec()
        .unwrap_ok();

    let id = submit(
        r#"
    Can i have CE?
    "#,
    );
    let err = e2e::RequestBuilder::new()
        .action(&format!("/runs/{}", id))
        .method(apiserver_engine::test_util::Method::Delete)
        .user("cersei")
        .exec()
        .unwrap_err();
    apiserver_engine::test_util::check_error(&err, "AccessDenied");
}

#[test]
fn test_heavy_load() {
    participate();
    let count = 20;
    let mut codes = Vec::new();
    println!("making {} submits", count);
    for _ in 0..count {
        let id = submit(
            r#"
            #include <cstdio>
            using ll = long long;
            int main() {
                ll a, b;
                scanf("%lld %lld", &a, &b);
                printf("%lld\n", a+b);
            }
            "#,
        );
        codes.push(id);
    }
    const BUDGET: u32 = 3;
    const INITIAL_BUDGET: u32 = 10;
    let mut budget = INITIAL_BUDGET;
    while !codes.is_empty() {
        std::thread::sleep(std::time::Duration::from_secs(3));
        let mut new_codes = Vec::new();
        for i in codes {
            let st = poll_status(i);
            if st == "QUEUE" {
                new_codes.push(i);
                continue;
            }
            println!("{} done", i);
            budget = std::cmp::max(budget, BUDGET);
            assert_eq!(st, "ACCEPTED");
        }
        println!("--- now {} running ---", new_codes.len());
        println!("{:?}", &new_codes);
        codes = new_codes;
        if budget == 0 {
            panic!("Timeout");
        }
        budget -= 1;
    }
}
