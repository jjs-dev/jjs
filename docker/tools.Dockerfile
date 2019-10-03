FROM ubuntu:19.04
RUN apt-get update -y && apt-get install -y libpq-dev build-essential postgresql-client-common postgresql-client-11
ADD bin/jjs-ppc /bin/jjs-ppc
ADD bin/jjs-userlist /bin/jjs-userlist
ADD bin/jjs-cli /bin/jjs-cli
ADD bin/jjs-setup /bin/jjs-setup
ADD bin/jjs-env-check /bin/jjs-env-check
ADD bin/jjs-cleanup /bin/jjs-cleanup