use std::{collections::HashMap, path::Path};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("openssl command missing")]
    OpensslMissing,
    #[error("openssl version is unsupported")]
    OpensslVersionUnsupported,
    #[error("child process failed: {command}")]
    Exec { command: String },
}

#[derive(Copy, Clone)]
enum ItemState {
    OnlyCert,
    OnlyKey,
    Both,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
enum ItemKind {
    CertificateAuthority,
    Invoker,
    Root,
}

impl ItemKind {
    fn all() -> impl Iterator<Item = ItemKind> {
        const ALL: &[ItemKind] = &[ItemKind::CertificateAuthority, ItemKind::Invoker, ItemKind::Root];
        ALL.iter().copied()
    }

    fn file_stem(self) -> &'static str {
        match self {
            ItemKind::CertificateAuthority => "ca",
            ItemKind::Invoker => "invoker",
            ItemKind::Root => "root"
        }
    }
}

struct CertsState {
    items: HashMap<ItemKind, ItemState>,
}

#[derive(Copy, Clone)]
pub struct Context<'a> {
    pub data_dir: &'a Path,
    pub can_create_ca: bool,
}

async fn get_state(cx: Context<'_>) -> Result<CertsState, Error> {
    let mut items = HashMap::new();
    let pki_dir = cx.data_dir.join("etc/pki");
    for kind in ItemKind::all() {
        let certificate_path = pki_dir.join(format!("{}.crt", kind.file_stem()));
        let key_path = pki_dir.join(format!("{}.key", kind.file_stem()));
        let has_cert = certificate_path.exists();
        let has_key = key_path.exists();
        let state = match (has_cert, has_key) {
            (true, true) => Some(ItemState::Both),
            (true, false) => Some(ItemState::OnlyCert),
            (false, true) => Some(ItemState::OnlyKey),
            (false, false) => None,
        };
        if let Some(state) = state {
            items.insert(kind, state);
        }
    }

    Ok(CertsState { items })
}

pub struct Certs<'a> {
    cx: Context<'a>,
    state: CertsState,
}

impl std::fmt::Display for Certs<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for kind in ItemKind::all() {
            let state = match self.state.items.get(&kind) {
                Some(ItemState::Both) => "cert+key",
                Some(ItemState::OnlyCert) => "cert",
                Some(ItemState::OnlyKey) => "key",
                None => "missing",
            };
            write!(f, "{}: {}; ", kind.file_stem(), state)?;
        }
        Ok(())
    }
}

async fn check_openssl() -> Result<(), Error> {
    let mut cmd = tokio::process::Command::new("openssl");
    cmd.arg("version").arg("-v");

    let out = cmd.output().await.map_err(|_| Error::OpensslMissing)?;
    if !out.status.success() {
        return Err(Error::OpensslMissing);
    }
    let out = String::from_utf8(out.stdout).map_err(|_| Error::OpensslVersionUnsupported)?;
    let out = out
        .split_whitespace()
        .nth(1)
        .ok_or(Error::OpensslVersionUnsupported)?;
    println!("Found openssl: {}", &out);
    if out < "1.1.0" {
        return Err(Error::OpensslVersionUnsupported);
    }
    Ok(())
}

async fn exec_cmd(mut c: tokio::process::Command) -> Result<(), Error> {
    c.status()
        .await
        .map_err(drop)
        .and_then(|st| if st.success() { Ok(()) } else { Err(()) })
        .map_err(|_| Error::Exec {
            command: format!("{:?}", c),
        })?;
    Ok(())
}

#[async_trait::async_trait]
impl<'a> crate::Component for Certs<'a> {
    type Error = Error;

    fn name(&self) -> &'static str {
        "pki keys & certificates"
    }

    async fn state(&self) -> Result<crate::StateKind, Self::Error> {
        let mut ok = true;
        let mut generatable = self.cx.can_create_ca;
        for k in ItemKind::all() {
            let v = self.state.items.get(&k).copied();
            if matches!(k, ItemKind::CertificateAuthority) {
                ok = ok && matches!(v, Some(ItemState::Both | ItemState::OnlyCert));
                generatable =
                    generatable || matches!(v, Some(ItemState::Both | ItemState::OnlyKey));
            } else {
                ok = ok && matches!(v, Some(ItemState::Both));
            }
        }
        if ok {
            Ok(crate::StateKind::UpToDate)
        } else if generatable {
            Ok(crate::StateKind::Upgradable)
        } else {
            Ok(crate::StateKind::Errored)
        }
    }

    async fn upgrade(&self) -> Result<(), Self::Error> {
        check_openssl().await?;
        let have_ca_key = matches!(
            self.state.items.get(&ItemKind::CertificateAuthority),
            Some(ItemState::OnlyKey | ItemState::Both)
        );
        let base = self.cx.data_dir.join("etc/pki");
        let ca_key_path = base.join("ca.key");
        if !have_ca_key {
            println!("Creating CA");
            // at first, we create key
            let mut cmd_create_key = tokio::process::Command::new("openssl");
            cmd_create_key.arg("genrsa");
            cmd_create_key.arg("-out").arg(&ca_key_path);
            cmd_create_key.arg("4096");
            exec_cmd(cmd_create_key).await?;
        }
        let have_ca_cert = matches!(
            self.state.items.get(&ItemKind::CertificateAuthority),
            Some(ItemState::OnlyCert | ItemState::Both)
        );
        let ca_cert_path = base.join("ca.crt");
        if !have_ca_cert {
            println!("Creating CA certificate");
            // now, we create self-signed certificate
            let mut cmd = tokio::process::Command::new("openssl");
            cmd.arg("req").arg("-x509").arg("-new");
            cmd.arg("-key").arg(&ca_key_path);
            cmd.arg("-days").arg("365");
            cmd.arg("-subj")
                .arg("/C=ZZ/ST=AA/L=loc/O=org/OU=ch/CN=jjs-ca/emailAddress=nope@example.com");
            cmd.arg("-out").arg(&ca_cert_path);
            exec_cmd(cmd).await?;
        }
        println!("Creating end entity certificates");
        for kind in ItemKind::all() {
            if matches!(kind, ItemKind::CertificateAuthority) {
                continue;
            }
            if matches!(self.state.items.get(&kind), Some(ItemState::Both)) {
                continue;
            }
            let ee_key_path = base.join(format!("{}.key", kind.file_stem()));
            let mut cmd_create_key = tokio::process::Command::new("openssl");
            cmd_create_key.arg("genrsa");
            cmd_create_key.arg("-out").arg(&ee_key_path);
            cmd_create_key.arg("4096");
            exec_cmd(cmd_create_key).await?;

            let ee_csr_path = base.join(format!("{}.csr", kind.file_stem()));
            let mut cmd_create_csr = tokio::process::Command::new("openssl");
            cmd_create_csr.arg("req").arg("-new");
            cmd_create_csr.arg("-key").arg(&ee_key_path);
            cmd_create_csr.arg("-out").arg(&ee_csr_path);
            cmd_create_csr.arg("-subj").arg(format!(
                "/C=ZZ/ST=AA/L=loc/O=org/OU=ch/CN={}/emailAddress=nope@example.com",
                kind.file_stem()
            ));
            exec_cmd(cmd_create_csr).await?;

            let ee_crt_path = base.join(format!("{}.crt", kind.file_stem()));
            let mut cmd_sign_csr = tokio::process::Command::new("openssl");
            cmd_sign_csr.arg("x509").arg("-req");
            cmd_sign_csr.arg("-in").arg(&ee_csr_path);
            cmd_sign_csr
                .arg("-CA")
                .arg(&ca_cert_path)
                .arg("-CAkey")
                .arg(&ca_key_path);
            cmd_sign_csr.arg("-CAcreateserial");
            cmd_sign_csr.arg("-days").arg("60");
            cmd_sign_csr.arg("-out").arg(&ee_crt_path);

            exec_cmd(cmd_sign_csr).await?;
        }

        Ok(())
    }
}

pub async fn analyze<'a>(cx: Context<'a>) -> Result<Certs<'a>, Error> {
    let state = get_state(cx).await?;
    Ok(Certs { cx, state })
}
