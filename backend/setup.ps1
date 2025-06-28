# Shop Saver XML Processor Setup Script
# Run this script to set up the development environment

Write-Host "=== Shop Saver XML Processor Setup ===" -ForegroundColor Green

# Check if Rust is installed
Write-Host "Checking Rust installation..." -ForegroundColor Yellow
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    $rustVersion = cargo --version
    Write-Host "✓ Rust found: $rustVersion" -ForegroundColor Green
} else {
    Write-Host "✗ Rust not found. Please install Rust from https://rustup.rs/" -ForegroundColor Red
    exit 1
}

# Check if PostgreSQL is accessible
Write-Host "Checking PostgreSQL..." -ForegroundColor Yellow
try {
    $pgVersion = psql --version 2>$null
    if ($pgVersion) {
        Write-Host "✓ PostgreSQL client found: $pgVersion" -ForegroundColor Green
    } else {
        Write-Host "⚠ PostgreSQL client not found in PATH. Make sure PostgreSQL is installed and accessible." -ForegroundColor Yellow
    }
} catch {
    Write-Host "⚠ PostgreSQL client not found. Make sure PostgreSQL is installed." -ForegroundColor Yellow
}

# Create .env file from example if it doesn't exist
if (!(Test-Path ".env")) {
    Write-Host "Creating .env file from example..." -ForegroundColor Yellow
    Copy-Item ".env.example" ".env"
    Write-Host "✓ .env file created. Please edit it with your database credentials." -ForegroundColor Green
} else {
    Write-Host "✓ .env file already exists." -ForegroundColor Green
}

# Check dependencies
Write-Host "Installing/updating Rust dependencies..." -ForegroundColor Yellow
cargo check
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Dependencies checked successfully." -ForegroundColor Green
} else {
    Write-Host "✗ Dependency check failed. Please check Cargo.toml" -ForegroundColor Red
    exit 1
}

Write-Host "`n=== Setup Complete! ===" -ForegroundColor Green
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "1. Edit .env file with your PostgreSQL credentials" -ForegroundColor White
Write-Host "2. Create PostgreSQL database: CREATE DATABASE shop_saver;" -ForegroundColor White
Write-Host "3. Run the server: cargo run" -ForegroundColor White
Write-Host "4. Place XML files in the watch directory" -ForegroundColor White
