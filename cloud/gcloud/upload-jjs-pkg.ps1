#/usr/bin/env powershell

param(
    [Parameter(Mandatory = $true)]
    [String]$BucketName,
    [Parameter(Mandatory = $true)]
    [String]$ArchivePath = "../../target/jjs.tgz"
)

$BucketFileName = "jjs.tgz"

gsutil cp $ArchivePath "gs://${BucketName}/${BucketFileName}"