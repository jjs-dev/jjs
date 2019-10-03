FROM ubuntu:19.04
RUN apt-get update -y && apt-get install -y libpq-dev
ADD bin/jjs-frontend /bin/jjs-frontend
CMD /bin/jjs-frontend