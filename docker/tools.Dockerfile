FROM jjs-env
RUN apt-get update -y && apt-get install -y libpq-dev build-essential postgresql-client-common postgresql-client-11
COPY bin/jjs-ppc bin/jjs-userlist bin/jjs-cli bin/jjs-setup bin/jjs-env-check bin/jjs-cleanup bin/jjs-configure-toolchains /bin/
