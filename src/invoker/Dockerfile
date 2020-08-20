FROM debian:stable-slim
RUN apt-get update -y && apt-get install -y libssl-dev ca-certificates
ENV JJS_DATA=/data
ENV JJS_AUTH_DATA=/auth/authdata.yaml
ENV RUST_BACKTRACE=1
COPY jjs-invoker /bin/jjs-invoker
ENTRYPOINT ["jjs-invoker"]
