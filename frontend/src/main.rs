#![feature(box_syntax)]

use thrift::{
    transport::{
        TFramedReadTransportFactory,
        TFramedWriteTransportFactory,
    },
    protocol::{
        TCompactInputProtocolFactory,
        TCompactOutputProtocolFactory,
    }
};

struct ApiContext {
    db: db::Db,
}

struct ApiContextProvider {}

impl ApiContextProvider {
    fn create(&self) -> ApiContext {

        let db_conn = postgres::Connection::connect("postgres://jjs:internal@localhost",
                                                        postgres::TlsMode::None).unwrap();
        let db = db::Db {
            submissions: Box::new(db::submission::PgSubmissions::new(box db_conn))
        };
        let t = ApiContext {
            db,
        };
        t
    }


    fn provide(&self) -> ApiContext {
        //TODO use thread-local cache
        self.create()
    }
}

struct Api {
    ctx_provider: ApiContextProvider,
}

impl frontend_api::JjsServiceSyncHandler for Api {
    fn handle_anon(&self) -> thrift::Result<frontend_api::AuthToken> {
        let s = "_".to_string();
        let buf = s.into_bytes();
        Ok(
            frontend_api::AuthToken {
                buf,
            }
        )
    }

    fn handle_simple(&self, params: frontend_api::SimpleAuthParams) -> thrift::Result<frontend_api::AuthToken> {
        let s = format!("${}", &params.login);
        let buf = s.into_bytes();
        Ok(
            frontend_api::AuthToken {
                buf,
            }
        )
    }

    fn handle_drop(&self, _token: frontend_api::AuthToken) -> thrift::Result<()> {
//TODO implement
        Ok(())
    }

    fn handle_submit(&self, params: frontend_api::SubmitDeclaration) -> thrift::Result<frontend_api::SubmissionId> {
        let ctx = self.ctx_provider.provide();
        let s8n = ctx.db.submissions.create_submission(&params.toolchain);
        Ok(s8n.id as i64)
    }

    fn handle_ping(&self, buf: String) -> thrift::Result<String> {
        Ok(buf)
    }
}

fn main() {
    let port = 1779;
    let listen_address = format!("127.0.0.1:{}", port);

    let i_tran_factory = TFramedReadTransportFactory::new();
    let i_prot_factory = TCompactInputProtocolFactory::new();

    let o_tran_factory = TFramedWriteTransportFactory::new();
    let o_prot_factory = TCompactOutputProtocolFactory::new();


    let processor = frontend_api::JjsServiceSyncProcessor::new(
        Api {
            ctx_provider: ApiContextProvider {}
        }
    );

    let mut server = thrift::server::TServer::new(
        i_tran_factory,
        i_prot_factory,
        o_tran_factory,
        o_prot_factory,
        processor,
        1,
    );

    println!("JJS api frontend is listening on {}", &listen_address);
    server.listen(&listen_address).unwrap();
}
