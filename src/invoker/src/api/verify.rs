// based on `AllowAnyAuthenticatedClient`
// from https://github.com/ctz/rustls/blob/bdaf35b2cc6c6679d3c40f70bdbb9cb0c83ba421/rustls/src/verify.rs
/*
pub(super) struct ClientAuthentifier {
    roots: rustls::RootCertStore,
}

impl ClientAuthentifier {
    pub(super) async fn new() -> anyhow::Result<ClientAuthentifier> {}
}

static SUPPORTED_SIG_ALGS: &[&webpki::SignatureAlgorithm] = &[
    &webpki::ECDSA_P256_SHA256,
    &webpki::ECDSA_P256_SHA384,
    &webpki::ECDSA_P384_SHA256,
    &webpki::ECDSA_P384_SHA384,
    &webpki::RSA_PSS_2048_8192_SHA256_LEGACY_KEY,
    &webpki::RSA_PSS_2048_8192_SHA384_LEGACY_KEY,
    &webpki::RSA_PSS_2048_8192_SHA512_LEGACY_KEY,
    &webpki::RSA_PKCS1_2048_8192_SHA256,
    &webpki::RSA_PKCS1_2048_8192_SHA384,
    &webpki::RSA_PKCS1_2048_8192_SHA512,
    &webpki::RSA_PKCS1_3072_8192_SHA384,
];

impl rustls::ClientCertVerifier for ClientAuthentifier {
    fn offer_client_auth(&self) -> bool {
        true
    }

    fn client_auth_mandatory(&self, sni: Option<&webpki::DNSName>) -> Option<bool> {
        Some(true)
    }

    fn client_auth_root_subjects(
        &self,
        sni: Option<&webpki::DNSName>,
    ) -> Option<rustls::DistinguishedNames> {
        Some(self.roots.get_subjects())
    }

    fn verify_client_cert(
        &self,
        presented_certs: &[rustls::Certificate],
        sni: Option<&webpki::DNSName>,
    ) -> Result<rustls::ClientCertVerified, rustls::TLSError> {
        if presented_certs.is_empty() {
            return Err(rustls::TLSError::NoCertificatesPresented);
        }
        let end_entity_cert = webpki::EndEntityCert::from(&presented_certs[0].0)
            .map_err(rustls::TLSError::WebPKIError)?;
        let trust_chain: Vec<&[u8]> = presented_certs
            .iter()
            .skip(1)
            .map(|cert| cert.0.as_ref())
            .collect();
        let trust_roots: Vec<webpki::TrustAnchor> = self
            .roots
            .roots
            .iter()
            .map(|anchor| anchor.to_trust_anchor())
            .collect();
        let now = webpki::Time::try_from(std::time::SystemTime::now())
            .map_err(|_| rustls::TLSError::FailedToGetCurrentTime)?;

        end_entity_cert.verify_is_valid_tls_client_cert(
            SUPPORTED_SIG_ALGS,
            &webpki::TLSClientTrustAnchors(&trust_roots),
            &trust_chain,
            now,
        ).map_err(rustls::TLSError::WebPKIError)?;

        end_entity_cert.verify_signature(signature_alg, msg, signature);

        Ok(rustls::ClientCertVerified::assertion())
    }
}
*/
