# JJS Judge
Judge is program that can actually evaluate user submissions and value them.
## Design
### Request lifecycle
On input, Judge receives `JudgeRequest`s different _request providers_. For example, it can be
Judge HTTP RPC API. Judge loads toolchain and problem, mentioned in request, using `ToolchainLoader`
and `ProblemLoader` respectively. Judge then starts special program called `Valuer`. Usually it will
be `svaluer` from the JJS distribution. Valuer determines tests submission should be tested on.
It also scores the run. Score and judging details are returned.
