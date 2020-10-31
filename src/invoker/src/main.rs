struct RpcHandler;

impl rpc::Handler for RpcHandler {
    
}

fn main() {
    let mut server = rpc::RouterBuilder::new();
    server.add_route(RpcHandler);
    let server = server.build().as_make_service();
    let server = hyper
}
