# Use this script as starting point


$OutDir = "/tmp/jjs/opt"

function Invoke-StraceLog {
    param(
        [String]$Prefix,
        [String]$LogPath
    )
    $StraceJSON = "$Prefix-str.json"
    Get-Content -Path $Strace | b3 > $StraceJSON
    $FileList = "$Prefix-list.json"
    cargo run -- "--data" $StraceJSON "--format" json "--dest" $FileList "--skip" "/dev" "--skip" "/home"
    $Files = Get-Content $FileList | ConvertFrom-Json
    $Files += "/lib64/ld-linux-x86-64.so.2"
    foreach ($File in $Files ) {
        $OutPath = "$OutDir$File"
        mkdir -p $OutPath 2>&1 | Out-Null
        rmdir $OutPath 2>&1 | Out-Null
        cp -a $File $OutPath
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

    Write-Output $Program > "$Prefix-prog.cpp"
    $Strace = "$Prefix-str.txt"
    strace -f -o $Strace -- g++ "$Prefix-prog.cpp" -o /dev/null
    Invoke-StraceLog -Prefix $Prefix -LogPath $Strace
}

function Bash {
    $Prefix="$GlobalDataRoot/bash"
    $Strace = "$Prefix-str.txt"
    strace -f -o $Strace -- bash -c "ls 2>&1" 
    Invoke-StraceLog -Prefix $Prefix -LogPath $Strace
}

Gcc 
Bash