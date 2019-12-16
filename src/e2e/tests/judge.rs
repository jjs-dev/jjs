mod util;
use std::convert::TryInto;

fn submit(code: &str) -> i32 {
    let code = base64::encode(code);

    let resp = util::RequestBuilder::new()
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
    let resp = util::RequestBuilder::new()
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
        if item.pointer("/id").unwrap().as_i64().unwrap() == id as i64 {
            return item
                .pointer("/status/code")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
        }
    }
    panic!("Run with id {} not found", id);
}

fn send_check_status(run_code: &str, correct_status: &str) {
    let id = submit(run_code);
    loop {
        let status = poll_status(id);
        if status == "QUEUE_JUDGE" {
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
     int a, b;
     scanf("%d %d", &a, &b);
     printf("%d\n", a+b);
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
     int a,b;
     scanf("%d %d", &a, &b);
     printf("%d\n", a-b);
 }       
        "#,
        "PARTIAL_SOLUTION",
    )
}

#[test]
fn test_non_privileged_user_cannot_see_non_their_runs() {
    util::RequestBuilder::new()
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
    let err = util::RequestBuilder::new()
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
    dbg!(&err);
    assert!(
        err.pointer("/extensions/errorCode")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("AccessDenied")
    );
}
