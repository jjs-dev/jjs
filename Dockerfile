FROM rust as build
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /usr/src/jjs
COPY . .
WORKDIR invoker
RUN cargo build --target=x86_64-unknown-linux-musl --release

FROM alpine
WORKDIR /usr/bin
COPY --from=build /usr/src/jjs/target/release/invoker .

#COPY target/release/invoker /usr/bin
#RUN chmod +x /usr/bin/invoker
#CMD /usr/bin/invoker