param([String]$GccVersion = 8,
    [String]$GccTarget = "x86_64-linux-gnu",
    [String]$Sysroot = "/tmp/jjs/opt")

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
    $DestPath = "$Sysroot/usr/lib/gcc/$GccTarget/$GccVersion/$ToolName"
    Copy-Item -Path $SrcPath -Destination $DestPath
}

New-Item -Path "$Sysroot/usr/lib/gcc/$GccTarget/$GccVersion" -ItemType Directory  | Out-Null

CopyTool "cc1"
CopyTool "cc1plus"