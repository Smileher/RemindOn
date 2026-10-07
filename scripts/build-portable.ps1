[CmdletBinding()]
param(
  [ValidateSet('x64', 'arm64')]
  [string]$Architecture = 'x64'
)

$ErrorActionPreference = 'Stop'
$repository = Split-Path -Parent $PSScriptRoot
$originalStoreBuild = $env:REMINDON_STORE_BUILD
Push-Location $repository
try {
  if (-not $IsWindows) { throw 'Windows portable builds must run on Windows.' }
  $version = (Get-Content -LiteralPath 'package.json' -Raw -Encoding UTF8 | ConvertFrom-Json).version
  $tauriVersion = (Get-Content -LiteralPath 'src-tauri/tauri.conf.json' -Raw -Encoding UTF8 | ConvertFrom-Json).version
  $cargoVersion = [regex]::Match((Get-Content -LiteralPath 'src-tauri/Cargo.toml' -Raw -Encoding UTF8), '(?m)^version = "([^"]+)"').Groups[1].Value
  if ($version -notmatch '^\d+\.\d+\.\d+$' -or $tauriVersion -ne $version -or $cargoVersion -ne $version) {
    throw 'package.json, tauri.conf.json and Cargo.toml must have the same valid version.'
  }
  $target = 'x86_64-pc-windows-msvc'
  if ($Architecture -eq 'arm64') { $target = 'aarch64-pc-windows-msvc' }
  $env:REMINDON_STORE_BUILD = '0'
  & pnpm tauri build --target $target --no-bundle --ci -- --locked
  if ($LASTEXITCODE -ne 0) { throw "Portable executable build failed for $Architecture." }

  $output = Join-Path $repository "src-tauri/target/portable/$version"
  New-Item -ItemType Directory -Path $output -Force | Out-Null
  $portable = Join-Path $output "RemindOn_${version}_${Architecture}_portable.exe"
  $archive = Join-Path $output "RemindOn_${version}_${Architecture}_portable.zip"
  Copy-Item -LiteralPath "src-tauri/target/$target/release/remindon.exe" -Destination $portable -Force
  try {
    Compress-Archive -LiteralPath $portable -DestinationPath $archive -CompressionLevel Optimal -Force
    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    [System.IO.File]::WriteAllText("$archive.sha256", "$hash`n", [System.Text.Encoding]::ASCII)
  } finally {
    Remove-Item -LiteralPath $portable -Force
  }
  Write-Host "Portable ZIP: $archive"
} finally {
  $env:REMINDON_STORE_BUILD = $originalStoreBuild
  Pop-Location
}
