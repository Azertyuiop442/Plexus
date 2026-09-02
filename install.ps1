Write-Host "======================================================" -ForegroundColor Cyan
Write-Host "          Installing Plexus on Windows                " -ForegroundColor Cyan
Write-Host "======================================================" -ForegroundColor Cyan

Write-Host "-> Checking Nerd Font installation..." -ForegroundColor Yellow
$FontInstalled = Get-ItemProperty -Path "HKLM:\Software\Microsoft\Windows NT\CurrentVersion\Fonts", "HKCU:\Software\Microsoft\Windows NT\CurrentVersion\Fonts" -ErrorAction SilentlyContinue | Get-Member -MemberType NoteProperty | Where-Object { $_.Name -like "*Nerd Font*" }

if (-not $FontInstalled) {
    Write-Host "   Installing JetBrainsMono Nerd Font via winget..." -ForegroundColor Yellow
    winget install -e --id DEVCOM.JetBrainsMonoNerdFont --silent --accept-source-agreements --accept-package-agreements 2>$null
} else {
    Write-Host "   Nerd Font already detected." -ForegroundColor Green
}

Write-Host "-> Checking Rust & Cargo..." -ForegroundColor Yellow
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Host "   Cargo not found. Installing Rustup via winget..." -ForegroundColor Yellow
    winget install -e --id Rustlang.Rustup --silent --accept-source-agreements --accept-package-agreements 2>$null
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")
}

$InstallDir = Join-Path $HOME ".commandcode\mods\cc-dashboard"
if ((Test-Path ".\Cargo.toml") -and (Test-Path ".\src\mux.rs")) {
    $WorkDir = (Get-Location).Path
} elseif (Test-Path (Join-Path $InstallDir ".git")) {
    Write-Host "-> Pulling latest release in $InstallDir..." -ForegroundColor Yellow
    git -C $InstallDir fetch --quiet origin public 2>$null
    git -C $InstallDir checkout --quiet public 2>$null
    git -C $InstallDir pull --ff-only origin public 2>$null
    $WorkDir = $InstallDir
} else {
    Write-Host "-> Cloning Plexus into $InstallDir..." -ForegroundColor Yellow
    $ParentDir = Split-Path -Parent $InstallDir
    if (-not (Test-Path $ParentDir)) { New-Item -ItemType Directory -Path $ParentDir -Force | Out-Null }
    git clone --depth 1 -b public https://github.com/Azertyuiop442/Plexus.git $InstallDir
    $WorkDir = $InstallDir
}
Set-Location $WorkDir

Write-Host "-> Compiling release binaries..." -ForegroundColor Yellow
cargo build --release

Write-Host "-> Deploying to $HOME\.commandcode\bin\..." -ForegroundColor Yellow
$BinDir = Join-Path $HOME ".commandcode\bin"
if (-not (Test-Path $BinDir)) {
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
}

Copy-Item "target\release\plexus.exe" -Destination "$BinDir\plexus.exe" -Force -ErrorAction SilentlyContinue
Copy-Item "target\release\cc-mux.exe" -Destination "$BinDir\cc-mux.exe" -Force -ErrorAction SilentlyContinue

Write-Host "======================================================" -ForegroundColor Cyan
Write-Host " [OK] Plexus successfully installed on Windows!       " -ForegroundColor Green
Write-Host "    - Standalone: Run 'plexus.exe'                    " -ForegroundColor White
Write-Host "    - In Command Code: Type '/dashboard'              " -ForegroundColor White
Write-Host "======================================================" -ForegroundColor Cyan
