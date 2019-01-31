FROM rustlang/rust:nightly
ARG concurrency=4
RUN rustup component add clippy
RUN rustup target add x86_64-unknown-linux-musl
RUN apt update -y
RUN apt install -y automake bison flex g++ git libboost-all-dev libevent-dev libssl-dev libtool make pkg-config
WORKDIR /thrift
RUN git clone https://github.com/apache/thrift src
WORKDIR /thrift/src
RUN git checkout v0.12.0
RUN ./bootstrap.sh
RUN ./configure --without-java --without-swift --without-cpp --without-qt4 --without-qt \
    --without-c_glib --without-csharp --without-java --without-erlang --without-nodejs --without-nodets \
    --without-lua --without-python --without-perl --without-php --without-php_extension \
    --without-dart --without-ruby --without-haskell --without-go --without-cl --without-haxe \
    --without-dotnetcore --without-d --disable-tutorial  --prefix=/opt/thrift
RUN make -j${concurrency}
RUN make install
