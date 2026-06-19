$ErrorActionPreference = "Stop"

$url = "https://github.com/protocolbuffers/protobuf/releases/download/v25.1/protoc-25.1-win64.zip"
$zipPath = "$env:TEMP\protoc.zip"
$extractPath = "$env:USERPROFILE\.protoc"

Write-Host "Downloading protoc from $url..."
Invoke-WebRequest -Uri $url -OutFile $zipPath

Write-Host "Extracting to $extractPath..."
If (Test-Path $extractPath) {
    Remove-Item -Recurse -Force $extractPath
}
Expand-Archive -Path $zipPath -DestinationPath $extractPath -Force

Write-Host "Done."
