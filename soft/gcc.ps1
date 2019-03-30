param([String]$GccVersion = 8,
    [String]$GccTarget = "x86_64-linux-gnu",
    [String]$Sysroot = "/tmp/jjs/opt")

Set-StrictMode -Version Latest

function GetTool {
    param([String]$ToolName)
    "/usr/lib/gcc/$GccTarget/$GccVersion/$ToolName"
}
$Path_cc1plus = GetTool "cc1plus"
if (!(Test-Path $Path_cc1plus)) {
    Write-Error "Error: couldn't find cc1plus. Is's probably means params were incorrect"
    exit 1
}

function CopyTool {
    param([String]$ToolName)
    $SrcPath = GetTool $ToolName
    Set-Location $PSScriptRoot
    cargo run -- "--with=$SrcPath" "--root=$Sysroot"
}

function CopyFile {
    param([String]$Path)
    $DestPath = "$Sysroot$Path"
    Copy-Item -Path $Path -Destination $DestPath
}

function CopyHeader {
    param([String]$HeaderName)
    $Dest = "$Sysroot/$HeaderName"
    New-Item -ItemType File -Path $Dest -Force | Out-Null
    Copy-Item -Path "$HeaderName" -Destination $Dest | Out-Null
}
function Flatten($a) {
    ,@($a | ForEach-Object { $_ })
}

Set-Location $PSScriptRoot

New-Item -Path "$Sysroot/usr/lib/gcc/$GccTarget/$GccVersion" -ItemType Directory  | Out-Null

CopyTool "cc1"
CopyTool "cc1plus"
CopyTool "liblto_plugin.so"
cargo run -- "--with=gcc" "--with=g++" "--with=as" "--with=ld" "--root=$Sysroot"
$CrtObjectDir = "/usr/lib/$GccTarget/"
CopyFile "$CrtObjectDir/Scrt1.o"
CopyFile "$CrtObjectDir/crti.o"
CopyFile "$CrtObjectDir/crtn.o"
CopyTool crtbeginS.o
CopyTool crtendS.o
#cargo run -- "--root=$Sysroot" "--deb=g++-8" "--deb=gcc-8"

#In order to determine headers path and copy them, we compile sample program

$Program = @'
#include <bits/stdc++.h>
#include <unistd.h>
int main() {
    return 0;
}
'@
$DepInfo = $Program |  g++ -x c++ - -M

$DepInfo = "$DepInfo".Substring(2)
$DepInfo = "$DepInfo".Replace('\', ' ') -split ' ' | Where-Object {$_.Trim() -ne ""}
$DepInfo | ForEach-Object { CopyHeader $_.Trim() }
$DIL = $DepInfo.Length
Write-Host "$DIL header files copied"

# Copy libraries
$LibDir = "/usr/lib/$GccTarget"
$GccLibDir = "/usr/lib/gcc/$GccTarget/$GccVersion"
$Libs = @("$LibDir/libm.a" , "$LibDir/libc.a", "$GccLibDir/libgcc.a", "$GccLibDir/libgcc_s.so", "$GccLibDir/libgcc_s.so.1", `
    "$GccLibDir/libstdc++.a", "/usr/lib/x86_64-linux-gnu/libm-2.28.a", `
    "/usr/lib/x86_64-linux-gnu/libmvec.a")
foreach ($Lib in $Libs) {
    Write-Host "Copying $Lib"
    CopyFile $Lib
}
