// To start server, we need to know some free port.
// Even if there is a way to get this information, it would
// suffer from race conditions.
// That's why, we simply select random port and try using it.
// 20 iterations give negligible probality of failure.
const BIND_ATTEMPTS: usize = 20;

#[tracing::instrument]
pub(crate) async fn create_server(
    cancel: tokio::sync::CancellationToken,
) -> anyhow::Result<(tokio::sync::oneshot::Receiver<()>, rpc::Client)> {
    // TODO provide way to customize port or port range
    tracing::info!("launching server");
    let mut last_error = None;
    for _ in 0..BIND_ATTEMPTS {
        let port: u16 = rand::random();
        match pps_server::serve(port, cancel.clone()).await {
            Ok(rx) => {
                let endpoint = format!("http://127.0.0.1:{}", port);
                let client = rpc::Client::new(rpc::ReqwestEngine::new(), endpoint);
                return Ok((rx, client));
            }
            Err(err) => {
                tracing::warn!(error=?err, "bind attempt unsuccessful");
                last_error = Some(err)
            }
        }
    }
    Err(last_error.expect("BIND_ATTEMPTS != 0"))
}
