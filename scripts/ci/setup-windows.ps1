$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
Set-StrictMode -Version Latest

$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vs = & $vswhere -latest -products '*' -property installationPath
if (-not $vs) { throw 'Visual Studio C++ build tools were not found' }
& "$vs\Common7\Tools\Launch-VsDevShell.ps1" -Arch $env:SPANDSP_CI_ARCH -HostArch amd64 -SkipAutomaticLocation

$vcpkg = Join-Path $env:RUNNER_TEMP 'spandsp-vcpkg'
git clone --filter=blob:none https://github.com/microsoft/vcpkg.git $vcpkg
git -C $vcpkg checkout 04a9d8e5212d01ee1dd9478eadd9caade4f8b0d4
& "$vcpkg\bootstrap-vcpkg.bat" -disableMetrics
$installed = Join-Path $env:RUNNER_TEMP 'spandsp-vcpkg-installed'
$triplet = "$env:SPANDSP_CI_ARCH-windows"
& "$vcpkg\vcpkg.exe" install --triplet $triplet --host-triplet $triplet `
    tiff libjpeg-turbo pkgconf "--x-install-root=$installed"
$prefix = Join-Path $installed $triplet
$pkgconf = Get-ChildItem "$prefix\tools" -Recurse -Filter pkgconf.exe | Select-Object -First 1
if (-not $pkgconf) { throw 'vcpkg did not install pkgconf' }
$clang = (Get-Command clang.exe -ErrorAction Stop).Source
$clangxx = (Get-Command clang++.exe -ErrorAction Stop).Source

$settings = @{
    CC = $clang
    CXX = $clangxx
    LIBCLANG_PATH = (Split-Path $clang)
    PKG_CONFIG = $pkgconf.FullName
    PKG_CONFIG_PATH = "$prefix\lib\pkgconfig;$prefix\share\pkgconfig"
    SPANDSP_VCPKG_PREFIX = $prefix
    PATH = "$prefix\bin;$env:PATH"
}
foreach ($name in @('INCLUDE', 'LIB', 'LIBPATH', 'WindowsSdkDir', 'WindowsSDKVersion', 'VCToolsRedistDir')) {
    $settings[$name] = [Environment]::GetEnvironmentVariable($name)
}
foreach ($entry in $settings.GetEnumerator()) {
    if (-not $entry.Value) { throw "Missing Windows build setting $($entry.Key)" }
    "$($entry.Key)=$($entry.Value)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
}
