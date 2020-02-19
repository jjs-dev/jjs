use std::convert::TryInto;

fn submit(code: &str) -> i32 {
    let code = base64::encode(code);

    let resp = e2e::RequestBuilder::new()
        .operation(
            r#"
mutation Submit($runCode: String!) {
  submitSimple(toolchain: "g++", problem: "A", runCode: $runCode, contest: "TODO") {
    id
  }
}
    "#,
        )
        .var("runCode", &serde_json::Value::from(code))
        .exec()
        .unwrap_ok();
    let resp = resp.pointer("/submitSimple/id").unwrap();
    resp.as_i64().unwrap().try_into().unwrap()
}

fn poll_status(id: i32) -> String {
    let resp = e2e::RequestBuilder::new()
        .operation(
            r#"
query GetRuns {
  runs{
    id,
    status {
      code
    }
  }
}        
        "#,
        )
        .exec()
        .unwrap_ok();
    let resp = resp.pointer("/runs").unwrap().as_array().unwrap();
    for item in resp {
        // dbg!(&item);
        if item.pointer("/id").unwrap().as_i64().unwrap() == id as i64 {
            let status = item.pointer("/status").unwrap();
            return status
                .as_object()
                .map(|s| s.get("code").unwrap().as_str().unwrap().to_string())
                .unwrap_or_else(|| "QUEUE".to_string());
        }
    }
    panic!("Run with id {} not found", id);
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
    e2e::RequestBuilder::new()
        .operation(
            r#"
mutation CreateUsrs {
  createUser(login:"cersei", groups: [], password:"") {
    id
  }
}
            "#,
        )
        .exec()
        .unwrap_ok();

    let id = submit(
        r#"
    Can i have CE?
    "#,
    );
    let err = e2e::RequestBuilder::new()
        .operation(
            r#"
    
mutation DeleteRun($runId: Int!) {
  modifyRun(id:$runId, delete: true)
}
    "#,
        )
        .var("runId", &serde_json::Value::from(id))
        .user("cersei")
        .exec()
        .unwrap_errs()
        .into_iter()
        .next()
        .unwrap();
    assert!(
        err.pointer("/extensions/errorCode")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("AccessDenied")
    );
}

#[test]
fn test_heavy_load() {
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
            assert_eq!(st, "ACCEPTED");
        }
        println!("--- now {} running ---", new_codes.len());
        println!("{:?}", &new_codes);
        codes = new_codes;
    }
}
