FROM debian:stable-slim
RUN apt-get update -y && apt-get install -y libssl-dev
COPY jjs-cli /bin/jjs-cli
ENV JJS_AUTH_DATA=/auth/authdata.yaml
VOLUME ["/auth"]
ENTRYPOINT ["jjs-cli"]
