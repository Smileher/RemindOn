[CmdletBinding()]
param(
  [switch]$LocalTest,
  [switch]$AllArchitectures
)

$ErrorActionPreference = 'Stop'
$repository = Split-Path -Parent $PSScriptRoot
$identityName = '54317Smileher.RemindOn'
$publisher = 'CN=426E8CF5-3861-440D-B400-CDB0323C5FD4'
$publisherDisplayName = 'Smileher'
$originalStoreBuild = $env:REMINDON_STORE_BUILD
$originalPath = $env:PATH
Push-Location $repository
try {
  $appVersion = (Get-Content -LiteralPath 'package.json' -Raw -Encoding UTF8 | ConvertFrom-Json).version
  $tauriVersion = (Get-Content -LiteralPath 'src-tauri/tauri.conf.json' -Raw -Encoding UTF8 | ConvertFrom-Json).version
  if ($appVersion -notmatch '^[1-9]\d*\.\d+\.\d+$' -or $tauriVersion -ne $appVersion) {
    throw 'package.json and tauri.conf.json must have the same valid Store version.'
  }
  $version = "$appVersion.0"
  $root = Join-Path $repository "src-tauri/target/store/$version"
  $uploadPackages = Join-Path $root 'upload-packages'
  $uploadOutput = Join-Path $root 'upload'
  $iconPath = Join-Path $repository 'src-tauri/icons/icon.png'
  $targets = @(@{ Architecture = 'x64'; Target = 'x86_64-pc-windows-msvc' })
  if ($AllArchitectures) {
    $targets += @{ Architecture = 'arm64'; Target = 'aarch64-pc-windows-msvc' }
    if (-not (Get-Command clang.exe -ErrorAction SilentlyContinue)) {
      $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
      if (Test-Path -LiteralPath $vswhere -PathType Leaf) {
        $clang = & $vswhere -products '*' -latest -find 'VC\Tools\Llvm\x64\bin\clang.exe' | Select-Object -First 1
        if ($clang) { $env:PATH = "$(Split-Path $clang);$env:PATH" }
      }
      if (-not (Get-Command clang.exe -ErrorAction SilentlyContinue)) {
        throw 'Install Visual Studio LLVM/Clang for ARM64 Store builds.'
      }
    }
  }
  $packageName = "RemindOn_${version}_x64.msix"
  if ($AllArchitectures) { $packageName = "RemindOn_${version}_bundle.msixbundle" }

  if ($LocalTest) {
    $friendlyName = 'RemindOn local Store test'
    $certificate = Get-ChildItem Cert:\CurrentUser\My | Where-Object {
      $_.FriendlyName -eq $friendlyName -and $_.Subject -eq $publisher -and $_.HasPrivateKey -and $_.NotAfter -gt (Get-Date).AddDays(1)
    } | Sort-Object NotAfter -Descending | Select-Object -First 1
    if (-not $certificate) {
      $certificate = New-SelfSignedCertificate -Type CodeSigningCert -Subject $publisher -FriendlyName $friendlyName -CertStoreLocation Cert:\CurrentUser\My -KeyExportPolicy Exportable -HashAlgorithm SHA256
    }
    $trustedPeople = Get-ChildItem Cert:\LocalMachine\TrustedPeople | Where-Object Thumbprint -EQ $certificate.Thumbprint
    if (-not $trustedPeople) {
      $certificateDirectory = Join-Path $repository 'src-tauri/target/store-certificate'
      New-Item -ItemType Directory -Path $certificateDirectory -Force | Out-Null
      $certificatePath = Join-Path $certificateDirectory 'RemindOn-local-test.cer'
      Export-Certificate -Cert $certificate -FilePath $certificatePath -Force | Out-Null
      throw "Install $certificatePath into Local Machine / Trusted People, then rerun this command."
    }
  }

  $env:REMINDON_STORE_BUILD = '1'
  foreach ($item in $targets) {
    & pnpm tauri build --target $item.Target --no-bundle --ci -- --locked
    if ($LASTEXITCODE -ne 0) { throw "Store executable build failed for $($item.Architecture)." }
    $executable = Join-Path $repository "src-tauri/target/$($item.Target)/release/remindon.exe"
    & "$PSScriptRoot/package-msix.ps1" -Architecture $item.Architecture -Version $version -IdentityName $identityName -Publisher $publisher -PublisherDisplayName $publisherDisplayName -SourceExecutable $executable -IconPath $iconPath -OutputDirectory $uploadPackages
    if ($LASTEXITCODE -ne 0) { throw "MSIX packaging failed for $($item.Architecture)." }
  }
  if ($AllArchitectures) {
    & "$PSScriptRoot/bundle-msix.ps1" -Version $version -InputDirectory $uploadPackages -OutputDirectory $uploadOutput
    if ($LASTEXITCODE -ne 0) { throw 'Store bundle packaging failed.' }
  } else {
    New-Item -ItemType Directory -Path $uploadOutput -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $uploadPackages $packageName) -Destination $uploadOutput -Force
  }

  if ($LocalTest) {
    $tempPfx = Join-Path ([IO.Path]::GetTempPath()) "remindon-$([guid]::NewGuid()).pfx"
    try {
      $password = "$([guid]::NewGuid().ToString('N'))$([guid]::NewGuid().ToString('N'))"
      $securePassword = ConvertTo-SecureString $password -AsPlainText -Force
      Export-PfxCertificate -Cert $certificate -FilePath $tempPfx -Password $securePassword | Out-Null
      $testPackages = Join-Path $root 'local-test-packages'
      $testOutput = Join-Path $root 'local-test'
      foreach ($item in $targets) {
        $executable = Join-Path $repository "src-tauri/target/$($item.Target)/release/remindon.exe"
        & "$PSScriptRoot/package-msix.ps1" -Architecture $item.Architecture -Version $version -IdentityName $identityName -Publisher $publisher -PublisherDisplayName $publisherDisplayName -SourceExecutable $executable -IconPath $iconPath -OutputDirectory $testPackages -CertificatePath $tempPfx -CertificatePassword $password
        if ($LASTEXITCODE -ne 0) { throw "Local MSIX signing failed for $($item.Architecture)." }
      }
      if ($AllArchitectures) {
        & "$PSScriptRoot/bundle-msix.ps1" -Version $version -InputDirectory $testPackages -OutputDirectory $testOutput -CertificatePath $tempPfx -CertificatePassword $password
        if ($LASTEXITCODE -ne 0) { throw 'Local bundle signing failed.' }
      } else {
        New-Item -ItemType Directory -Path $testOutput -Force | Out-Null
        Copy-Item -LiteralPath (Join-Path $testPackages $packageName) -Destination $testOutput -Force
      }
    } finally {
      Remove-Item -LiteralPath $tempPfx -Force -ErrorAction SilentlyContinue
    }
  }

  Write-Output "Store upload: $(Join-Path $uploadOutput $packageName)"
  if ($LocalTest) {
    Write-Output "Local install: $(Join-Path $testOutput $packageName)"
  }
} finally {
  $env:REMINDON_STORE_BUILD = $originalStoreBuild
  $env:PATH = $originalPath
  Pop-Location
}
