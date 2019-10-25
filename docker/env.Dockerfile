FROM ubuntu:19.04
RUN apt-get update -y && apt-get install -y build-essential gcc-9 g++-9 python3 openjdk-11-jdk libunwind8 busybox  --no-install-recommends
ADD https://jjs-dist.s3.amazonaws.com/ktrace.deb /tmp/ktrace.deb
RUN dpkg -i /tmp/ktrace.deb
