use client::{prelude::Sendable as _, ApiClient};
use either::Either;

#[derive(clap::Clap)]
pub struct Opt {
    /// problem code, e.g. "A"
    #[clap(long, short = 'p')]
    problem: String,
    #[clap(long, short = 't')]
    toolchain: String,
    #[clap(long, short = 'f')]
    filename: String,
    #[clap(long, short = 'c')]
    contest: String,
    /// Watch for judging finish
    #[clap(long, short = 'w')]
    watch: bool,
}

struct Run {
    inner: client::models::Run,
    current_score: i64,
    current_test: i64,
}

impl Run {
    fn new(inner: client::models::Run) -> Run {
        Run {
            inner,
            current_score: 0,
            current_test: 0,
        }
    }

    fn into_inner(self) -> client::models::Run {
        self.inner
    }

    async fn poll(
        &mut self,
        client: &ApiClient,
    ) -> anyhow::Result<Either<client::models::Run, client::models::LiveStatus>> {
        let status = client::models::LiveStatus::get_run_live_status()
            .run_id(&self.inner.id)
            .send(client)
            .await?
            .object;
        if let Some(ct) = &status.current_test {
            self.current_test = *ct as i64;
        }
        if let Some(ls) = &status.current_score {
            self.current_score = *ls as i64;
        }
        println!(
            "score = {}, running on test {}",
            self.current_score, self.current_test
        );
        if status.finished {
            println!("judging finished");
            let run = client::models::Run::get_run()
                .run_id(&self.inner.id)
                .send(client)
                .await?;
            return Ok(Either::Left(run.object));
        }
        Ok(Either::Right(status))
    }
}
async fn make_submit(
    client: &ApiClient,
    contest: &str,
    problem: &str,
    code: &str,
    toolchain: &str,
) -> anyhow::Result<Run> {
    let created_run = client::models::RunSubmitSimpleParams::submit_run()
        .code(code)
        .contest(contest)
        .problem(problem)
        .toolchain(toolchain)
        .send(client)
        .await?;
    println!("submitted: id={}", created_run.object.id);
    Ok(Run::new(created_run.object))
}

pub async fn exec(opt: Opt, api: &client::ApiClient) -> anyhow::Result<()> {
    let data = std::fs::read(&opt.filename).expect("Couldn't read file");
    let code = base64::encode(&data);

    let run = make_submit(api, &opt.contest, &opt.problem, &code, &opt.toolchain).await?;
    let _results = if opt.watch {
        let mut run = run;
        loop {
            if let Either::Left(done) = run.poll(api).await? {
                break done;
            }
            tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
        }
    } else {
        run.into_inner()
    };

    Ok(())
}
