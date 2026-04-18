#Requires -Version 5.1
<#
.SYNOPSIS
  jira-cli installer for Windows.
.EXAMPLE
  iwr -useb https://raw.githubusercontent.com/zhiyue/jira-cli/main/install.ps1 | iex
#>
param(
  [string]$Version = "",
  [string]$InstallDir = "",
  [string]$BaseUrl = "",
  [string]$Repo = "zhiyue/jira-cli"
)
$ErrorActionPreference = "Stop"

if (-not $BaseUrl) { $BaseUrl = "https://github.com/$Repo/releases" }

# Detect arch
$arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else {
  throw "unsupported architecture: $env:PROCESSOR_ARCHITECTURE"
}
$target = "$arch-pc-windows-msvc"

# Resolve version
if (-not $Version) {
  $resp = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
  $Version = $resp.tag_name
}

# Resolve install dir
if (-not $InstallDir) {
  $InstallDir = Join-Path $env:USERPROFILE ".local\bin"
  New-Item -Force -ItemType Directory $InstallDir | Out-Null
}

$archiveName = "jira-cli-$Version-$target.zip"
$url = "$BaseUrl/download/$Version/$archiveName"
$shaUrl = "$url.sha256"

Write-Host "==> Installing jira-cli $Version ($target) to $InstallDir"

$tmp = New-Item -Force -ItemType Directory (Join-Path $env:TEMP "jira-cli-install-$(Get-Random)")
try {
  $zipPath = Join-Path $tmp $archiveName
  Write-Host "==> Downloading $url"
  Invoke-WebRequest -UseBasicParsing $url -OutFile $zipPath

  Write-Host "==> Verifying SHA256"
  $shaFile = "$zipPath.sha256"
  Invoke-WebRequest -UseBasicParsing $shaUrl -OutFile $shaFile
  $expected = (Get-Content $shaFile -Raw).Trim().Split()[0]
  $actual = (Get-FileHash -Algorithm SHA256 $zipPath).Hash.ToLower()
  if ($expected.ToLower() -ne $actual) {
    throw "checksum mismatch: expected $expected, got $actual"
  }

  Expand-Archive -Force $zipPath -DestinationPath $tmp
  $bin = Get-ChildItem -Recurse -Filter "jira-cli.exe" $tmp | Select-Object -First 1
  if (-not $bin) { throw "jira-cli.exe not found in archive" }
  Copy-Item -Force $bin.FullName (Join-Path $InstallDir "jira-cli.exe")

  Write-Host "==> Installed: $InstallDir\jira-cli.exe"
  & (Join-Path $InstallDir "jira-cli.exe") --version
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
