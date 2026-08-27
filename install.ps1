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

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
Set-Location $ScriptDir

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
