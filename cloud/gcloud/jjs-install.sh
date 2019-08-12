#!/usr/bin/env bash

export PATH=$PATH:/snap/bin

env

gsutil cp gs://"${JJS_BUCKET}"/jjs.tgz /tmp/jjs.tar

sudo mkdir /opt/jjs

tar -xvf /tmp/jjs.tar --directory /opt/jjs