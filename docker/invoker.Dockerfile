FROM jjs-env
RUN apt-get update -y && apt-get install -y libpq-dev
ADD bin/jjs-invoker /bin/jjs-invoker
COPY bin /opt/jjs/bin/
CMD /bin/jjs-invoker
