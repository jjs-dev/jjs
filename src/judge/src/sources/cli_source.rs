use crate::controller::{InvocationFinishReason, JudgeRequestAndCallbacks, JudgeResponseCallbacks};
use anyhow::Context as _;
use judging_apis::{CliJudgeRequest, JudgeRequest};
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tracing::debug;
use uuid::Uuid;

async fn convert_request(cli_judge_request: CliJudgeRequest) -> anyhow::Result<JudgeRequest> {
    let run_source = tokio::fs::read(&cli_judge_request.run_source)
        .await
        .context("run_source not readable")?;
    Ok(JudgeRequest {
        revision: cli_judge_request.revision,
        toolchain_id: cli_judge_request.toolchain_id,
        problem_id: cli_judge_request.problem_id,
        request_id: cli_judge_request.request_id,
        run_source,
    })
}

#[derive(serde::Serialize)]
pub enum Message {
    Finish(FinishedMessage),
    Progress(ProgressMessage),
    LiveStatusUpdate(LsuMessage),
}
#[derive(serde::Serialize)]
pub struct FinishedMessage {
    invocation_id: Uuid,
    reason: &'static str,
}

#[derive(serde::Serialize)]
pub struct ProgressMessage {
    invocation_id: Uuid,
    header: judging_apis::JudgeOutcomeHeader,
}

#[derive(serde::Serialize)]
pub struct LsuMessage {
    invocation_id: Uuid,
    update: judging_apis::LiveStatusUpdate,
}

struct Callbacks;

#[async_trait::async_trait]
impl JudgeResponseCallbacks for Callbacks {
    async fn set_finished(
        &self,
        invocation_id: Uuid,
        reason: InvocationFinishReason,
    ) -> anyhow::Result<()> {
        let reason = match reason {
            InvocationFinishReason::CompileError => "CompileError",
            InvocationFinishReason::TestingDone => "TestingDone",
            InvocationFinishReason::Fault => "Fault",
        };
        print_message(Message::Finish(FinishedMessage {
            invocation_id,
            reason,
        }))
        .await
    }

    async fn add_outcome_header(
        &self,
        invocation_id: Uuid,
        header: judging_apis::JudgeOutcomeHeader,
    ) -> anyhow::Result<()> {
        print_message(Message::Progress(ProgressMessage {
            invocation_id,
            header,
        }))
        .await
    }

    async fn deliver_live_status_update(
        &self,
        invocation_id: Uuid,
        update: judging_apis::LiveStatusUpdate,
    ) -> anyhow::Result<()> {
        print_message(Message::LiveStatusUpdate(LsuMessage {
            invocation_id,
            update,
        }))
        .await
    }
}

async fn read_worker_iteration(
    req_tx: &mut async_mpmc::Sender<JudgeRequestAndCallbacks>,
    stdin_reader: &mut tokio::io::BufReader<tokio::io::Stdin>,
) -> anyhow::Result<()> {
    let mut line = String::new();
    let ret = stdin_reader
        .read_line(&mut line)
        .await
        .context("failed to read line")?;
    if ret == 0 {
        tokio::time::delay_for(std::time::Duration::from_secs(30)).await;
    }
    let task = serde_json::from_str(&line).context("unparseable CliInvokeTask")?;
    debug!("got {:?}", &task);
    let request = convert_request(task).await?;
    let judge_request_and_cbs = JudgeRequestAndCallbacks {
        request,
        callbacks: Arc::new(Callbacks),
    };
    req_tx.send(judge_request_and_cbs);
    Ok(())
}

async fn print_message(msg: Message) -> anyhow::Result<()> {
    let msg = serde_json::to_string(&msg).context("serialization error")?;
    println!("{}", msg);
    Ok(())
}

pub async fn run(
    mut req_tx: async_mpmc::Sender<JudgeRequestAndCallbacks>,
    cancel: tokio::sync::CancellationToken,
) {
    let mut reader = tokio::io::BufReader::new(tokio::io::stdin());

    let run_fut = async move {
        loop {
            if let Err(err) = read_worker_iteration(&mut req_tx, &mut reader).await {
                eprintln!("read iteration failed: {:#}", err);
            }
        }
    };
    tokio::select! {
        _ = run_fut => (),
        _ = cancel.cancelled() => (),
    }
}
