FROM debian:stable-slim
# TODO: use rustls
RUN apt-get update -y && apt-get install -y libssl-dev
ENV JJS_AUTH_DATA=/auth/authdata.yaml JJS_PATH=/jtl
COPY ppc /ppc
VOLUME ["/auth"]
CMD ["/ppc"]