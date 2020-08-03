FROM debian:stable-slim
RUN apt-get update -y && apt-get install -y libssl-dev
COPY cli /cli
ENV JJS_AUTH_DATA=/auth/authdata.yaml
VOLUME ["/auth"]
CMD ["/cli"]
