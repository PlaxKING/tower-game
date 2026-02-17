# Fix UE5 5.3 - Simple patch
$filePath = "C:\Program Files\Epic Games\UE_5.3\Engine\Source\Runtime\Core\Public\Experimental\ConcurrentLinearAllocator.h"

Write-Host "Applying simple patch..." -ForegroundColor Cyan

# Restore from backup
$backupPath = "$filePath.backup"
if (Test-Path $backupPath) {
    Copy-Item $backupPath $filePath -Force
    Write-Host "Restored from backup" -ForegroundColor Gray
}

# Remove read-only
Set-ItemProperty -Path $filePath -Name IsReadOnly -Value $false

# Read file
$lines = Get-Content $filePath

# Find and replace line 31
for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '#elif __has_feature\(address_sanitizer\)') {
        $lines[$i] = '#else  // __has_feature not available in MSVC'
        Write-Host "Replaced line $($i+1)" -ForegroundColor Green
        break
    }
}

# Write back
$lines | Set-Content $filePath

Write-Host "Patch applied!" -ForegroundColor Green
Write-Host ""
Write-Host "Press any key..."
$null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
