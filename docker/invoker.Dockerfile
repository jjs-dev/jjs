FROM jjs-env
RUN apt-get update -y && apt-get install -y libpq-dev
ADD bin/jjs-invoker /bin/jjs-invoker
CMD /bin/jjs-invoker
