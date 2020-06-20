FROM debian:stable-slim
RUN apt-get update -y && apt-get install -y libssl-dev
ENV JJS_DATA=/data
ENV JJS_PATH=/no-such-jjs-path
COPY invoker /invoker
CMD ["/invoker"]