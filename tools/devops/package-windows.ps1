<#
.SYNOPSIS
  Build the Windows x64 app and the per-user NSIS installer on Windows.

.DESCRIPTION
  Runs on the `windows` Forgejo runner (native MSVC). Nothing here cross-builds:
  a Linux cross-build of gpui-pre is not possible, because its build script
  hands llvm-rc a manifest path relative to the crate root while running the
  compiler from the build output directory.

  SNACKPILOT_PRODUCT_VERSION overrides the version from
  src/ui/ios/project.yml; SNACKPILOT_PACKAGE_OUT overrides dist/windows.

  The installer is UNSIGNED: SmartScreen warns until the binary has reputation.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$RepoRoot = (Resolve-Path (Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) '..\..')).Path
$UiDir = Join-Path $RepoRoot 'src\ui'
$Nsi = Join-Path $UiDir 'desktop\windows\installer.nsi'
$Icon = Join-Path $UiDir 'desktop\windows\SnackPilot.ico'
$ProjectYml = Join-Path $UiDir 'ios\project.yml'

function Fail($message) {
    Write-Error "package-windows: $message" -ErrorAction Stop
}

function Info($message) {
    Write-Host "package-windows: $message"
}

function Yaml-Value($path, $key) {
    $match = Select-String -Path $path -Pattern "^\s*$key\s*:\s*`"?([^`"\r\n]+)`"?" |
        Select-Object -First 1
    if (-not $match) { Fail "missing $key in $path" }
    return $match.Matches[0].Groups[1].Value.Trim()
}

foreach ($tool in 'cargo', 'makensis') {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        Fail "missing tool: $tool (install NSIS 3 and put makensis on PATH; rustup for cargo)"
    }
}
foreach ($input in $Nsi, $Icon, $ProjectYml) {
    if (-not (Test-Path $input)) { Fail "missing packaging input: $input" }
}

$version = $env:SNACKPILOT_PRODUCT_VERSION
if (-not $version) { $version = Yaml-Value $ProjectYml 'MARKETING_VERSION' }
$build = $env:SNACKPILOT_BUILD
if (-not $build) { $build = Yaml-Value $ProjectYml 'CURRENT_PROJECT_VERSION' }

if ($version -notmatch '^\d+\.\d+\.\d+$') { Fail "invalid product version: '$version'" }
if ($build -notmatch '^\d+$') { Fail "invalid build number: '$build'" }

$outDir = if ($env:SNACKPILOT_PACKAGE_OUT) { $env:SNACKPILOT_PACKAGE_OUT } else { Join-Path $RepoRoot 'dist\windows' }
$setup = Join-Path $outDir "SnackPilot-$version-windows-x64-setup.exe"
$receipt = Join-Path $outDir 'native-build-receipt.json'

Info "release $version ($build), target x86_64-pc-windows-msvc"
Info 'trust: UNSIGNED (cannot be Authenticode-signed here)'
Info "output: $outDir"

if (Test-Path $outDir) { Remove-Item -Recurse -Force $outDir }
New-Item -ItemType Directory -Path $outDir | Out-Null

Info 'building the desktop binary'
Push-Location $UiDir
try {
    $env:SNACKPILOT_PRODUCT_VERSION = $version
    $env:SNACKPILOT_BUILD = $build
    & cargo build --locked --release -p snackpilot-app --bin snackpilot-desktop --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { Fail "cargo build failed with exit code $LASTEXITCODE" }
}
finally {
    Pop-Location
}

$binDir = Join-Path $UiDir 'target\x86_64-pc-windows-msvc\release'
$bin = Join-Path $binDir 'snackpilot-desktop.exe'
if (-not (Test-Path $bin)) { Fail "release binary missing: $bin" }

Info 'building the per-user installer'
& makensis -V2 -INPUTCHARSET UTF8 `
    "-DVERSION=$version" "-DBUILD=$build" "-DSOURCE=$binDir" "-DOUT=$setup" "-DICON=$Icon" $Nsi
if ($LASTEXITCODE -ne 0) { Fail "makensis failed with exit code $LASTEXITCODE" }
if (-not (Test-Path $setup)) { Fail 'makensis produced no installer' }

$checksum = (Get-FileHash -Algorithm SHA256 -Path $setup).Hash.ToLowerInvariant()
"$checksum  $(Split-Path -Leaf $setup)" |
    Set-Content -Path "$setup.sha256" -Encoding ascii -NoNewline

@{
    platform        = 'windows'
    architecture    = 'x86_64'
    version         = $version
    build           = $build
    artifact_path   = $setup
    trust_state     = 'unsigned'
    checksum_scheme = 'sha256-file'
    checksum        = $checksum
} | ConvertTo-Json | Set-Content -Path $receipt -Encoding utf8

Info "artifact: $setup"
Info "sha256:   $checksum"
