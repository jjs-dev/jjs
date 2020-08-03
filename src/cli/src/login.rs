use anyhow::Context as _;
use std::path::PathBuf;

#[derive(clap::Clap)]
pub(crate) struct Opt {
    /// JJS apiserver endpoint. If not provided, will be requested on stdin.
    #[clap(long)]
    endpoint: Option<String>,
    /// File credentials should be written too.
    #[clap(long)]
    auth_data: Option<PathBuf>,
}

async fn ask(prompt: &str, default: Option<&str>) -> anyhow::Result<String> {
    let prompt = prompt.to_string();
    let default = default.map(ToString::to_string);
    tokio::task::spawn_blocking(|| -> anyhow::Result<String> {
        let mut inp = dialoguer::Input::new();
        inp.with_prompt(prompt);
        if let Some(def) = default {
            inp.default(def).show_default(true);
        }
        inp.interact().map_err(Into::into)
    })
    .await
    .unwrap()
}

pub(crate) async fn exec(opt: &Opt) -> anyhow::Result<()> {
    let endpoint = match &opt.endpoint {
        Some(ep) => ep.clone(),
        None => ask("JJS apiserver endpoint", Some("http://localhost:1779/")).await?,
    };
    let username = ask("Username", None).await?;
    let password = tokio::task::spawn_blocking(|| -> anyhow::Result<String> {
        let mut pass = dialoguer::Password::new();
        pass.with_prompt("Password");
        pass.interact().map_err(Into::into)
    })
    .await
    .unwrap()?;
    println!("Veryfying credentials");
    let ad = client::AuthData {
        endpoint: endpoint.clone(),
        auth: client::auth_data::AuthKind::LoginAndPassword(
            client::auth_data::AuthByLoginAndPassword {
                login: username.clone(),
                password: password.clone(),
            },
        ),
    };
    if let Err(err) = client::from_auth_data(ad.clone()).await {
        anyhow::bail!("Credentials invalid: {:#}", err);
    }
    let path = match &opt.auth_data {
        Some(p) => p.clone(),
        None => tokio::task::spawn_blocking(|| {
            let dirs =
                xdg::BaseDirectories::with_prefix("jjs").context("XDG initialization failed")?;

            dirs.place_config_file("authdata.yaml")
                .map_err(anyhow::Error::from)
        })
        .await
        .unwrap()?,
    };
    println!("Writing credentials to {}", path.display());
    let auth_data = serde_yaml::to_string(&ad)?;
    tokio::fs::write(path, auth_data).await?;

    Ok(())
}
