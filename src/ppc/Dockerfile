FROM debian:stable-slim
# TODO: use rustls
RUN apt-get update -y && apt-get install -y libssl-dev
ENV JJS_AUTH_DATA=/auth/authdata.yaml JJS_PATH=/jtl
COPY jjs-ppc /jjs-ppc
VOLUME ["/auth"]
ENTRYPOINT ["/jjs-ppc"]
