FROM debian:stable-slim
WORKDIR /opt/jjs
ENV JJS_AUTH_DATA=/auth/authdata.yaml JJS_PATH=/opt/jjs
COPY /jtl .
COPY /ppc ./bin
VOLUME ["/auth"]
