# Fix UE5 5.3 - Final patch
$filePath = "C:\Program Files\Epic Games\UE_5.3\Engine\Source\Runtime\Core\Public\Experimental\ConcurrentLinearAllocator.h"

Write-Host "Applying final patch..." -ForegroundColor Cyan

# Restore from backup
$backupPath = "$filePath.backup"
if (Test-Path $backupPath) {
    Copy-Item $backupPath $filePath -Force
    Write-Host "Restored from backup" -ForegroundColor Gray
}

# Remove read-only
Set-ItemProperty -Path $filePath -Name IsReadOnly -Value $false

# Read file as single string with explicit line endings
$content = [System.IO.File]::ReadAllText($filePath)

# Replace the problematic line
$oldPattern = '#elif __has_feature(address_sanitizer)'
$newPattern = '#else'

if ($content.Contains($oldPattern)) {
    $content = $content.Replace($oldPattern, $newPattern)
    [System.IO.File]::WriteAllText($filePath, $content)
    Write-Host "Patch applied successfully!" -ForegroundColor Green
} else {
    Write-Host "Pattern not found or already patched" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Press any key..."
$null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
