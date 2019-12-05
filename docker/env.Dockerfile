FROM ubuntu:19.04
RUN apt-get update -y && apt-get install -y build-essential gcc-9 g++-9 python3 openjdk-11-jdk libunwind8 busybox pkg-config libunwind-dev curl --no-install-recommends
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh /dev/stdin -y --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install lxtrace
