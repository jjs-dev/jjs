param([String]$GccVersion = 8,
    [String]$GccTarget = "x86_64-linux-gnu",
    [String]$Sysroot = "/tmp/jjs/opt")

Set-StrictMode -Version Latest

function Get-Tool {
    param([String]$ToolName)
    "/usr/lib/gcc/$GccTarget/$GccVersion/$ToolName"
}
$Path_cc1plus = Get-Tool "cc1plus"
if (!(Test-Path $Path_cc1plus)) {
    Write-Error "Error: couldn't find cc1plus. Is's probably means params were incorrect"
    exit 1
}

function Copy-Tool {
    param([String]$ToolName, [Switch]$WithDependencies)
    $SrcPath = Get-Tool $ToolName
    Set-Location $PSScriptRoot
    if ($WithDependencies) {
        cargo run -- "--bin=$SrcPath" "--root=$Sysroot"
    } else {
        Copy-Item -Path $SrcPath -Destination "$Sysroot/$ToolName"
    }
}

function Copy-File {
    param([String]$Path)
    $DestPath = "$Sysroot$Path"
    Copy-Item -Path $Path -Destination $DestPath
}

function Copy-Header {
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

Copy-Tool "cc1" -WithDependencies
Copy-Tool "cc1plus" -WithDependencies
Copy-Tool "liblto_plugin.so" -WithDependencies
cargo run -- "--bin=gcc" "--bin=g++" "--bin=as" "--bin=ld" "--root=$Sysroot"
$CrtObjectDir = "/usr/lib/$GccTarget/"
Copy-File "$CrtObjectDir/Scrt1.o"
Copy-File "$CrtObjectDir/crti.o"
Copy-File "$CrtObjectDir/crtn.o"
Copy-File "$CrtObjectDir/crt1.o"
Copy-File "/usr/lib/gcc/$GccTarget/$GccVersion/crtbeginT.o"
Copy-File "/usr/lib/gcc/$GccTarget/$GccVersion/libgcc_eh.a"
Copy-Tool crtbeginS.o
Copy-Tool crtendS.o
Copy-Tool crtend.o
Copy-Tool crtbegin.o
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
$DepInfo | ForEach-Object { Copy-Header $_.Trim() }
$DIL = $DepInfo.Length
Write-Host "$DIL header files copied"

# Copy libraries
$LibDir = "/usr/lib/$GccTarget"
$GccLibDir = "/usr/lib/gcc/$GccTarget/$GccVersion"
$Libs = @("$LibDir/libm.a" , "$LibDir/libc.a", "$GccLibDir/libgcc.a", "/lib/$GccTarget/libgcc_s.so.1", `
    "$GccLibDir/libstdc++.a", "/usr/lib/x86_64-linux-gnu/libm-2.28.a", `
    "/usr/lib/x86_64-linux-gnu/libmvec.a")
foreach ($Lib in $Libs) {
    Write-Host "Copying $Lib"
    Copy-File $Lib
}
