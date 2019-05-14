# Use this script as starting point
# prerequisities:
# $ cargo install --git https://github.com/mikailbag/copy-ln

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$OutDir = "/tmp/jjs/opt"

Remove-Item -Recurse $OutDir
New-Item -ItemType Directory $OutDir
function Invoke-StraceLog {
    param(
        [String]$Prefix,
        [String]$LogPath
    )
    $StraceJSON = "$Prefix-str.json"
    Get-Content -Path $Strace | python3 $PSScriptRoot/strace-parser.py > $StraceJSON
    $FileList = "$Prefix-list.json"
    cargo run "--package" soft -- "--data" $StraceJSON "--format" json "--dest" $FileList "--skip" "/dev" "--skip" "/home"
    $Files = Get-Content $FileList | ConvertFrom-Json
    $Files += "/lib64/ld-linux-x86-64.so.2"
    foreach ($File in $Files ) {
        $OutPath = "$OutDir$File"
        copy-ln "--file" $File "--prefix" $OutDir "--skip-exist"
    }
}
$GlobalDataRoot = "/tmp/jjs-soft"
New-Item -ItemType Directory -Path $GlobalDataRoot -ErrorAction SilentlyContinue | Out-Null
function Gcc {
    $Prefix="$GlobalDataRoot/g++"
    $Program = @'
    #include <bits/stdc++.h> 
    #include <iostream>      

    int main() {
    return 0;
    }
'@

    $ProgBin = "$Prefix-prog.elf"
    Write-Output $Program > "$Prefix-prog.cpp"
    $Strace = "$Prefix-str-build.txt"
    strace -f -o $Strace -- g++ "$Prefix-prog.cpp" -o $ProgBin
    Invoke-StraceLog -Prefix $Prefix -LogPath $Strace
    $Strace = "$Prefix-str-run.txt"
    strace -f  -o $Strace -- $ProgBin
    Invoke-StraceLog -Prefix $Prefix -LogPath $Strace
}

function Bash {
    $Prefix="$GlobalDataRoot/bash"
    $Strace = "$Prefix-str.txt"
    strace -f -o $Strace -- bash -c "busybox 2>&1" 
    Invoke-StraceLog -Prefix $Prefix -LogPath $Strace
}

Gcc 
Bash