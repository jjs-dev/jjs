#!/usr/bin/env powershell

param(
    [Parameter(Mandatory = $true)]
    [String]$InstanceName,
    [Parameter(Mandatory = $true)]
    [String]$InstaceTemplateName,
    [Parameter(Mandatory = $true)]
    [String]$BucketName
)
$ErrorActionPreference = "Stop"
gcloud compute instances create $InstanceName --source-instance-template $InstaceTemplateName

$PostgresInitScript = @'
create role jjs with password 'internal';
alter role jjs with login;
create database jjs;
grant all on all tables in schema public to jjs;
grant all on all sequences in schema public to jjs;
'@

$InstanceInitScript = @'
sudo apt-get update
sudo apt-get upgrade -y
sudo apt-get install postgresql-11 postgresql-client-11 -y
sudo su postgres -c "psql -f /tmp/pg-init-script.sql"

export JJS_BUCKET=${BucketName}
bash /tmp/jjs-install.sh
'@

$InstanceInitScript > /tmp/instance-init-script
$PostgresInitScript > /tmp/postgres-init-script
gcloud compute scp /tmp/instance-init-script ${InstanceName}:/tmp/init-script.sh
gcloud compute scp /tmp/postgres-init-script ${InstanceName}:/tmp/pg-init-script.sql
gcloud compute scp $PSScriptRoot/jjs-install.sh ${InstanceName}:/tmp/jjs-install.sh
gcloud compute ssh $InstanceName --command "bash /tmp/init-script.sh"