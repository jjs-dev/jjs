mod config;
mod graph_exec;
mod init;
mod invoke_util;
mod print_invoke_request;
mod transport;

#[derive(Clone)]
struct RpcHandler;

impl rpc::Handler<judging_apis::invoke::Invoke> for RpcHandler {
    type Error = anyhow::Error;
    type Fut = futures_util::future::BoxFuture<'static, Result<(), Self::Error>>;

    fn handle(
        self,
        request: rpc::UnaryRx<judging_apis::invoke::InvokeRequest>,
        response: rpc::UnaryTx<judging_apis::invoke::InvokeResponse>,
    ) -> Self::Fut {
        todo!()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut server = rpc::RouterBuilder::new();
    server.add_route(RpcHandler);
    let server = server.build().as_make_service();
    let incoming = transport::IncomingStdio::new();
    hyper::Server::builder(incoming).serve(server).await;
    Ok(())
}
