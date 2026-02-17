# Fix UE5 5.3 compatibility with VS 2022 14.44
# Run this script as Administrator

$filePath = "C:\Program Files\Epic Games\UE_5.3\Engine\Source\Runtime\Core\Public\Experimental\ConcurrentLinearAllocator.h"

Write-Host "Patching UE5 5.3 for VS 2022 14.44 compatibility..." -ForegroundColor Cyan

# Create backup
$backupPath = "$filePath.backup"
if (!(Test-Path $backupPath)) {
    Copy-Item $filePath $backupPath
    Write-Host "Backup created: $backupPath" -ForegroundColor Green
}

# Read file
$content = Get-Content $filePath -Raw

# Apply patch
$oldText = "#elif __has_feature(address_sanitizer)"
$newText = "#elif defined(__has_feature) && __has_feature(address_sanitizer)"

if ($content -match [regex]::Escape($oldText)) {
    $content = $content -replace [regex]::Escape($oldText), $newText
    Set-Content -Path $filePath -Value $content -NoNewline
    Write-Host "Patch applied successfully!" -ForegroundColor Green
    Write-Host "  Fixed line 31 to check if __has_feature is defined before using it" -ForegroundColor Gray
} else {
    Write-Host "Pattern not found (file may already be patched)" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Press any key to continue..."
$null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
