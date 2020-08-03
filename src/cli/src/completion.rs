/// Prints completion script to stdout.
/// Usually is should be used like this:
///
/// . completion.sh
#[derive(clap::Clap)]
pub(crate) struct Opt {
    /// Shell generate completions for.
    /// Supported: bash, fish, zsh, elvish, powershell|pwsh|ps
    #[clap(long, default_value = "bash")]
    shell: String,
}

fn generate<G: clap_generate::Generator>() {
    let mut app: clap::App = <crate::Opt as clap::derive::IntoApp>::into_app();
    clap_generate::generate::<G, _>(&mut app, "jjs-cli", &mut std::io::stdout());
}

pub(crate) fn exec(opt: &Opt) -> anyhow::Result<()> {
    match opt.shell.as_str() {
        "bash" => generate::<clap_generate::generators::Bash>(),
        "fish" => generate::<clap_generate::generators::Fish>(),
        "zsh" => generate::<clap_generate::generators::Zsh>(),
        "elvish" => generate::<clap_generate::generators::Elvish>(),
        "powershell" | "pwsh" | "ps" => generate::<clap_generate::generators::PowerShell>(),
        _ => anyhow::bail!("unsupported shell: {}", opt.shell),
    }
    Ok(())
}
