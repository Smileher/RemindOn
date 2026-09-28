[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][ValidateSet('x64', 'arm64')][string]$Architecture,
  [Parameter(Mandatory = $true)][ValidatePattern('^\d+\.\d+\.\d+\.\d+$')][string]$Version,
  [Parameter(Mandatory = $true)][string]$IdentityName,
  [Parameter(Mandatory = $true)][string]$Publisher,
  [Parameter(Mandatory = $true)][string]$SourceExecutable,
  [Parameter(Mandatory = $true)][string]$IconPath,
  [Parameter(Mandatory = $true)][string]$OutputDirectory,
  [Parameter(Mandatory = $true)][string]$CertificatePath,
  [Parameter(Mandatory = $true)][string]$CertificatePassword
)

$ErrorActionPreference = 'Stop'
foreach ($value in @($IdentityName, $Publisher, $SourceExecutable, $IconPath, $CertificatePath)) {
  if ([string]::IsNullOrWhiteSpace($value) -or -not (Test-Path -LiteralPath $value)) {
    if ($value -eq $IdentityName -or $value -eq $Publisher) { throw 'MSIX IdentityName and Publisher are required.' }
    throw "Required MSIX input does not exist: $value"
  }
}
if ([string]::IsNullOrWhiteSpace($CertificatePassword)) { throw 'MSIX certificate password is required.' }

$makeAppx = (Get-Command makeappx.exe -ErrorAction Stop).Source
$signtool = (Get-Command signtool.exe -ErrorAction Stop).Source
$root = Join-Path $env:RUNNER_TEMP "remindon-msix-$Architecture"
$stage = Join-Path $root 'package'
Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path (Join-Path $stage 'Assets') -Force | Out-Null

Copy-Item -LiteralPath $SourceExecutable -Destination (Join-Path $stage 'RemindOn.exe')
foreach ($name in @('StoreLogo.png', 'Square44x44Logo.png', 'Square150x150Logo.png')) {
  Copy-Item -LiteralPath $IconPath -Destination (Join-Path $stage "Assets\$name")
}

$manifest = @"
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10" xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10" xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedCapabilities">
  <Identity Name="$IdentityName" Publisher="$Publisher" Version="$Version" ProcessorArchitecture="$Architecture" />
  <Properties>
    <DisplayName>RemindOn</DisplayName>
    <PublisherDisplayName>ChenHe</PublisherDisplayName>
    <Description>RemindOn desktop reminders</Description>
    <Logo>Assets\StoreLogo.png</Logo>
  </Properties>
  <Resources><Resource Language="zh-CN" /></Resources>
  <Applications>
    <Application Id="RemindOn" Executable="RemindOn.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements AppListEntry="default" DisplayName="RemindOn" Description="RemindOn desktop reminders" Square44x44Logo="Assets\Square44x44Logo.png" Square150x150Logo="Assets\Square150x150Logo.png" />
    </Application>
  </Applications>
  <Capabilities><rescap:Capability Name="runFullTrust" /></Capabilities>
</Package>
"@
$manifestPath = Join-Path $stage 'AppxManifest.xml'
[IO.File]::WriteAllText($manifestPath, $manifest, [Text.Encoding]::UTF8)

New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$msixPath = Join-Path $OutputDirectory "RemindOn_${Version}_${Architecture}.msix"
& $makeAppx pack /d $stage /p $msixPath /o
if ($LASTEXITCODE -ne 0) { throw "MakeAppx failed with exit code $LASTEXITCODE" }
& $signtool sign /fd SHA256 /f $CertificatePath /p $CertificatePassword $msixPath
if ($LASTEXITCODE -ne 0) { throw "SignTool failed with exit code $LASTEXITCODE" }
& $signtool verify /pa $msixPath
if ($LASTEXITCODE -ne 0) { throw "SignTool verification failed with exit code $LASTEXITCODE" }
Write-Output $msixPath
