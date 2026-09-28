[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][ValidatePattern('^[1-9]\d*\.\d+\.\d+\.0$')][string]$Version,
  [Parameter(Mandatory = $true)][string]$InputDirectory,
  [Parameter(Mandatory = $true)][string]$OutputDirectory,
  [string]$CertificatePath,
  [string]$CertificatePassword
)

$ErrorActionPreference = 'Stop'
foreach ($component in $Version.Split('.')) {
  $number = [uint16]0
  if (-not [uint16]::TryParse($component, [ref]$number)) { throw 'MSIX version components must be between 0 and 65535.' }
}
if ($CertificatePath) {
  if (-not (Test-Path -LiteralPath $CertificatePath -PathType Leaf)) { throw 'MSIX signing certificate does not exist.' }
  if ([string]::IsNullOrWhiteSpace($CertificatePassword)) { throw 'MSIX certificate password is required.' }
} elseif ($CertificatePassword) {
  throw 'MSIX certificate password was provided without a certificate.'
}
$makeAppx = (Get-Command makeappx.exe -ErrorAction SilentlyContinue).Source
if (-not $makeAppx) {
  $sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
  $makeAppx = Get-ChildItem -LiteralPath $sdkRoot -Directory | Where-Object Name -Match '^\d+\.\d+\.\d+\.\d+$' |
    Sort-Object { [version]$_.Name } -Descending | ForEach-Object { Join-Path $_.FullName 'x64\makeappx.exe' } |
    Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
}
if (-not $makeAppx) { throw 'Install the Windows SDK with MakeAppx.exe before packaging.' }
$signtool = Join-Path (Split-Path $makeAppx) 'signtool.exe'
if ($CertificatePath -and -not (Test-Path -LiteralPath $signtool -PathType Leaf)) { throw 'SignTool.exe was not found in the Windows SDK.' }
$packages = @(Get-ChildItem -LiteralPath $InputDirectory -Filter '*.msix' -File)
if ($packages.Count -ne 2) { throw "Expected exactly two architecture MSIX packages, found $($packages.Count)." }
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$bundlePath = Join-Path $OutputDirectory "RemindOn_${Version}_bundle.msixbundle"
& $makeAppx bundle /d $InputDirectory /p $bundlePath /bv $Version /o
if ($LASTEXITCODE -ne 0) { throw "MakeAppx bundle failed with exit code $LASTEXITCODE" }
if ($CertificatePath) {
  & $signtool sign /fd SHA256 /f $CertificatePath /p $CertificatePassword $bundlePath
  if ($LASTEXITCODE -ne 0) { throw "SignTool failed with exit code $LASTEXITCODE" }
  & $signtool verify /pa $bundlePath
  if ($LASTEXITCODE -ne 0) { throw "SignTool verification failed with exit code $LASTEXITCODE" }
}
Write-Output $bundlePath
