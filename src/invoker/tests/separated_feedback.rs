#[tokio::test]
async fn separated_feedback() {
    let driver = invoker::sources::BackgroundSourceManager::create()
        .fork()
        .await;

    let id = uuid::Uuid::parse_str("fdfd0b03-4adb-4166-b10c-a3f3155b1067").unwrap();
    let run_dir = tempfile::TempDir::new().unwrap();
    let invocation_dir = tempfile::TempDir::new().unwrap();
    driver
        .add_task(invoker_api::InvokeTask {
            revision: 0,
            invocation_id: id,
            toolchain_id: "g++".to_string(),
            problem_id: "A".to_string(),
            run_dir: run_dir.path().to_path_buf(),
            invocation_dir: invocation_dir.path().to_path_buf(),
        })
        .await;
    // TODO write this test when cfg is rewritten
}
