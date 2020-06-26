FROM debian:stable-slim
WORKDIR /opt/jjs
ENV JJS_AUTH_DATA=/auth/authdata.yaml JJS_PATH=/opt/jjs PATH=/opt/jjs/bin:${PATH} \
  LIBRARY_PATH=/opt/jjs/lib:${LIBRARY_PATH} \
  CPLUS_INCLUDE_PATH=/opt/jjs/include:{$CPLUS_INCLUDE_PATH} \
  CMAKE_PREFIX_PATH=/opt/jjs/share/cmake:${CMAKE_PREFIX_PATH}
RUN apt-get update && apt-get install -y libssl-dev 
COPY /jtl .
COPY /ppc ./bin/jjs-ppc
COPY /cli ./bin/jjs-cli
COPY /svaluer ./bin/jjs-svaluer
VOLUME ["/auth"]
