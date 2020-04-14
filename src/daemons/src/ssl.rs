use once_cell::sync::Lazy;
use std::{
    any::Any,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Stored in request extenstions map.
/// Contains username (taken directly from certificate CN)
/// If certificate was missing, this key will contain None or will be missing.
#[derive(Clone, Debug)]
pub struct AuthenticatedUser(pub Option<String>);
mod authenticated_user_util {
    impl super::AuthenticatedUser {
        pub fn guard_name(&self, expected_name: &str) -> Result<(), actix_web::error::Error> {
            let ok = match self.0.as_ref() {
                Some(s) => s == expected_name,
                None => false,
            };
            if ok {
                Ok(())
            } else {
                Err(actix_web::error::ErrorUnauthorized(
                    "TLS client certificate missing, invalid or contains wrong CN",
                ))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CreateSslBuilderError {
    #[error("openssl error")]
    Openssl(#[from] openssl::error::ErrorStack),
    #[error("io error on {}", path.display())]
    Io {
        #[source]
        e: std::io::Error,
        path: PathBuf,
    },
}

#[derive(Eq, PartialEq)]
pub enum MutualAuthentication {
    Enabled,
    Required,
}

pub fn create_ssl_acceptor_builder(
    pki_base: &Path,
    auth: MutualAuthentication,
    name: &str,
) -> Result<openssl::ssl::SslAcceptorBuilder, CreateSslBuilderError> {
    let mut ssl_builder =
        openssl::ssl::SslAcceptor::mozilla_modern(openssl::ssl::SslMethod::tls())?;
    ssl_builder.set_certificate_chain_file(pki_base.join(format!("{}.crt", name)))?;
    ssl_builder.set_private_key_file(
        pki_base.join(format!("{}.key", name)),
        openssl::ssl::SslFiletype::PEM,
    )?;

    let ca_certificate_path = pki_base.join("ca.crt");
    let ca_certificate =
        std::fs::read(&ca_certificate_path).map_err(|e| CreateSslBuilderError::Io {
            e,
            path: ca_certificate_path,
        })?;

    let ca_certificate = openssl::x509::X509::from_pem(&ca_certificate)?;
    let mut client_store_builder = openssl::x509::store::X509StoreBuilder::new()?;
    client_store_builder.add_cert(ca_certificate)?;
    ssl_builder.set_verify_cert_store(client_store_builder.build())?;

    let mut verify_mode = openssl::ssl::SslVerifyMode::PEER;
    if matches!(auth, MutualAuthentication::Required) {
        verify_mode |= openssl::ssl::SslVerifyMode::FAIL_IF_NO_PEER_CERT;
    }
    // CN will be extracted later
    ssl_builder.set_verify(verify_mode);

    // disallow legacy (and potentially insecure) TLS versions
    ssl_builder.set_min_proto_version(Some(openssl::ssl::SslVersion::TLS1_2))?;
    Ok(ssl_builder)
}

fn try_get_authenticated_common_name(ssl: &openssl::ssl::SslRef) -> Option<String> {
    let peer_certificate = ssl.peer_certificate()?;
    // now we must check that certificate verification succeeded
    let verify_result = ssl.verify_result();
    if verify_result != openssl::x509::X509VerifyResult::OK {
        return None;
    }
    let subject_name = peer_certificate.subject_name();
    let entry = subject_name
        .entries_by_nid(openssl::nid::Nid::COMMONNAME)
        .next()?;
    let entry = entry.data();
    let name = entry.as_slice().to_vec();
    let name = String::from_utf8(name).ok()?;
    Some(name)
}

fn on_connect_hook(stream: &dyn std::any::Any) -> AuthenticatedUser {
    let stream = match stream
        .downcast_ref::<tokio_openssl::SslStream<openssl::ssl::SslStream<tokio::net::TcpStream>>>()
    {
        Some(stream) => stream,
        None => return dbg!(AuthenticatedUser(None)),
    };
    let stream = stream.get_ref();
    let ssl = stream.ssl();
    let authenticated_cn = try_get_authenticated_common_name(ssl);
    AuthenticatedUser(authenticated_cn)
}

static HOOK: Lazy<Arc<dyn Fn(&dyn Any) -> AuthenticatedUser + Send + Sync + 'static>> =
    Lazy::new(|| Arc::new(on_connect_hook));

pub fn make_on_connect_hook() -> Arc<dyn Fn(&dyn Any) -> AuthenticatedUser + Send + Sync + 'static>
{
    HOOK.clone()
}
