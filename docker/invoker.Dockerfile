FROM ubuntu:19.04
RUN apt-get update -y && apt-get install -y libpq-dev
ADD bin/jjs-invoker /bin/jjs-invoker
CMD /bin/jjs-invoker
