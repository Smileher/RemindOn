[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][ValidatePattern('^\d+\.\d+\.\d+\.\d+$')][string]$Version,
  [Parameter(Mandatory = $true)][string]$InputDirectory,
  [Parameter(Mandatory = $true)][string]$OutputDirectory,
  [Parameter(Mandatory = $true)][string]$CertificatePath,
  [Parameter(Mandatory = $true)][string]$CertificatePassword
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($CertificatePassword)) { throw 'MSIX certificate password is required.' }
$makeAppx = (Get-Command makeappx.exe -ErrorAction Stop).Source
$signtool = (Get-Command signtool.exe -ErrorAction Stop).Source
$packages = @(Get-ChildItem -LiteralPath $InputDirectory -Filter '*.msix' -File)
if ($packages.Count -ne 2) { throw "Expected exactly two architecture MSIX packages, found $($packages.Count)." }
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$bundlePath = Join-Path $OutputDirectory "RemindOn_${Version}_bundle.msixbundle"
& $makeAppx bundle /d $InputDirectory /p $bundlePath /o
if ($LASTEXITCODE -ne 0) { throw "MakeAppx bundle failed with exit code $LASTEXITCODE" }
& $signtool sign /fd SHA256 /f $CertificatePath /p $CertificatePassword $bundlePath
if ($LASTEXITCODE -ne 0) { throw "SignTool failed with exit code $LASTEXITCODE" }
& $signtool verify /pa $bundlePath
if ($LASTEXITCODE -ne 0) { throw "SignTool verification failed with exit code $LASTEXITCODE" }
Write-Output $bundlePath
