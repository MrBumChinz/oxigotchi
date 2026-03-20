# Oxigotchi RNDIS IP Setup
# Ensures 10.0.0.1/24 is always on the RNDIS adapter
# Run as: powershell -ExecutionPolicy Bypass -File setup_rndis_ip.ps1

$adapterName = Get-NetAdapter | Where-Object { $_.InterfaceDescription -like '*Raspberry*' -or $_.InterfaceDescription -like '*RNDIS*' -or $_.InterfaceDescription -like '*USB Ethernet*' } | Select-Object -First 1 -ExpandProperty Name

if (-not $adapterName) {
    Write-Host "No RNDIS adapter found"
    exit 1
}

$existing = Get-NetIPAddress -InterfaceAlias $adapterName -IPAddress "10.0.0.1" -ErrorAction SilentlyContinue
if (-not $existing) {
    New-NetIPAddress -InterfaceAlias $adapterName -IPAddress 10.0.0.1 -PrefixLength 24 -ErrorAction SilentlyContinue | Out-Null
    Write-Host "Added 10.0.0.1/24 to $adapterName"
} else {
    Write-Host "10.0.0.1/24 already on $adapterName"
}

# Enable IP forwarding for internet sharing
Set-NetIPInterface -InterfaceAlias $adapterName -Forwarding Enabled -ErrorAction SilentlyContinue
