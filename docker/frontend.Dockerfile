FROM jjs-env
RUN apt-get update -y && apt-get install -y libpq-dev libcurl3-gnutls
ADD bin/jjs-frontend /bin/jjs-frontend
CMD /bin/jjs-frontend