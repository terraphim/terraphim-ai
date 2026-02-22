$ErrorActionPreference = 'Stop'

$packageArgs = @{
  packageName   = 'terraphim-llm-proxy'
  fileType      = 'exe'
  file          = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)\terraphim-llm-proxy.exe"
  silentArgs    = ''
  validExitCodes= @(0)
  softwareName  = 'terraphim-llm-proxy*'
}

Install-ChocolateyPackage @packageArgs

# Install configuration file
$configPath = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)\config.example.toml"
$targetConfigPath = "$(Split-Path -parent (Get-Command terraphim-llm-proxy).Source)\config.toml"

if (-not (Test-Path $targetConfigPath)) {
    Copy-Item $configPath $targetConfigPath
}

Write-Host "Terraphim LLM Proxy installed successfully!"
Write-Host "Example configuration copied to: $targetConfigPath"
Write-Host ""
Write-Host "To get started:"
Write-Host "1. Edit the configuration file with your API keys"
Write-Host "2. Run: terraphim-llm-proxy --config $targetConfigPath"