FROM ubuntu:19.10
RUN apt-get update -y && apt-get install -y ca-certificates build-essential libunwind8 pkg-config libunwind-dev curl libcurl3-gnutls \
  libpq-dev build-essential postgresql-client-common postgresql-client-11 strace busybox \ 
   --no-install-recommends
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh /dev/stdin -y --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"
COPY . /opt/jjs
ENV PATH="/opt/jjs/bin:$PATH"
ENTRYPOINT []
