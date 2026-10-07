[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][ValidateSet('x64', 'arm64')][string]$Architecture,
  [Parameter(Mandatory = $true)][ValidatePattern('^[1-9]\d*\.\d+\.\d+\.0$')][string]$Version,
  [Parameter(Mandatory = $true)][string]$IdentityName,
  [Parameter(Mandatory = $true)][string]$Publisher,
  [string]$PublisherDisplayName = 'Smileher',
  [Parameter(Mandatory = $true)][string]$SourceExecutable,
  [Parameter(Mandatory = $true)][string]$IconPath,
  [Parameter(Mandatory = $true)][string]$OutputDirectory,
  [string]$CertificatePath,
  [string]$CertificatePassword
)

$ErrorActionPreference = 'Stop'
foreach ($value in @($IdentityName, $Publisher, $PublisherDisplayName)) {
  if ([string]::IsNullOrWhiteSpace($value)) { throw 'MSIX identity and publisher values are required.' }
}
foreach ($value in @($SourceExecutable, $IconPath)) {
  if (-not (Test-Path -LiteralPath $value -PathType Leaf)) { throw "Required MSIX input does not exist: $value" }
}
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
$makePri = Join-Path (Split-Path $makeAppx) 'makepri.exe'
if (-not (Test-Path -LiteralPath $makePri -PathType Leaf)) { throw 'MakePRI.exe was not found in the Windows SDK.' }
$signtool = Join-Path (Split-Path $makeAppx) 'signtool.exe'
if ($CertificatePath -and -not (Test-Path -LiteralPath $signtool -PathType Leaf)) { throw 'SignTool.exe was not found in the Windows SDK.' }
$root = Join-Path ([IO.Path]::GetTempPath()) "remindon-msix-$Architecture-$([guid]::NewGuid())"
$stage = Join-Path $root 'package'
New-Item -ItemType Directory -Path (Join-Path $stage 'Assets') -Force | Out-Null

Copy-Item -LiteralPath $SourceExecutable -Destination (Join-Path $stage 'RemindOn.exe')
foreach ($name in @('StoreLogo.png', 'Square44x44Logo.png', 'Square150x150Logo.png')) {
  $logo = Join-Path (Split-Path $IconPath) $name
  if (-not (Test-Path -LiteralPath $logo -PathType Leaf)) { throw "Required MSIX logo does not exist: $logo" }
  Copy-Item -LiteralPath $logo -Destination (Join-Path $stage "Assets\$name")
}

# Windows requires default, dark and light AppList variants to avoid an accent-color plate.
$sourceIcon = [Drawing.Bitmap]::new((Resolve-Path -LiteralPath $IconPath).Path)
try {
  foreach ($size in @(16, 20, 24, 30, 32, 36, 40, 44, 48, 60, 64, 72, 80, 96, 256)) {
    $bitmap = [Drawing.Bitmap]::new($size, $size, [Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
      $graphics.Clear([Drawing.Color]::Transparent)
      $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
      $graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::HighQuality
      $graphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
      $graphics.DrawImage($sourceIcon, 0, 0, $size, $size)
      foreach ($variant in @('', '_altform-unplated', '_altform-lightunplated')) {
        $bitmap.Save((Join-Path $stage "Assets\Square44x44Logo.targetsize-${size}${variant}.png"), [Drawing.Imaging.ImageFormat]::Png)
      }
    } finally {
      $graphics.Dispose()
      $bitmap.Dispose()
    }
  }
} finally {
  $sourceIcon.Dispose()
}

$IdentityName = [Security.SecurityElement]::Escape($IdentityName)
$Publisher = [Security.SecurityElement]::Escape($Publisher)
$PublisherDisplayName = [Security.SecurityElement]::Escape($PublisherDisplayName)
$manifest = @"
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10" xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10" xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities" IgnorableNamespaces="uap rescap">
  <Identity Name="$IdentityName" Publisher="$Publisher" Version="$Version" ProcessorArchitecture="$Architecture" />
  <Properties>
    <DisplayName>RemindOn</DisplayName>
    <PublisherDisplayName>$PublisherDisplayName</PublisherDisplayName>
    <Description>RemindOn desktop reminders</Description>
    <Logo>Assets\StoreLogo.png</Logo>
  </Properties>
  <Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.26100.0" /></Dependencies>
  <Resources><Resource Language="zh-CN" /><Resource Language="en-US" /></Resources>
  <Applications>
    <Application Id="RemindOn" Executable="RemindOn.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements AppListEntry="default" DisplayName="RemindOn" Description="RemindOn desktop reminders" BackgroundColor="transparent" Square44x44Logo="Assets\Square44x44Logo.png" Square150x150Logo="Assets\Square150x150Logo.png" />
    </Application>
  </Applications>
  <Capabilities><rescap:Capability Name="runFullTrust" /></Capabilities>
</Package>
"@
$manifestPath = Join-Path $stage 'AppxManifest.xml'
[IO.File]::WriteAllText($manifestPath, $manifest, [Text.Encoding]::UTF8)

# Qualified icon files need a PRI index so Windows can select the unplated variants.
$priConfig = Join-Path $root 'priconfig.xml'
& $makePri createconfig /cf $priConfig /dq en-US /pv 10.0.0 /o
if ($LASTEXITCODE -ne 0) { throw "MakePRI configuration failed with exit code $LASTEXITCODE" }
& $makePri new /pr $stage /cf $priConfig /mn $manifestPath /of (Join-Path $stage 'resources.pri') /o
if ($LASTEXITCODE -ne 0) { throw "MakePRI indexing failed with exit code $LASTEXITCODE" }

New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$msixPath = Join-Path $OutputDirectory "RemindOn_${Version}_${Architecture}.msix"
& $makeAppx pack /d $stage /p $msixPath /o
if ($LASTEXITCODE -ne 0) { throw "MakeAppx failed with exit code $LASTEXITCODE" }
if ($CertificatePath) {
  & $signtool sign /fd SHA256 /f $CertificatePath /p $CertificatePassword $msixPath
  if ($LASTEXITCODE -ne 0) { throw "SignTool failed with exit code $LASTEXITCODE" }
  & $signtool verify /pa $msixPath
  if ($LASTEXITCODE -ne 0) { throw "SignTool verification failed with exit code $LASTEXITCODE" }
}
Write-Output $msixPath
