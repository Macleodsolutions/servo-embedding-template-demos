$vite = Start-Process -FilePath "cmd.exe" -ArgumentList "/c yarn dev" -WorkingDirectory "$PSScriptRoot\ui" -PassThru -NoNewWindow

Start-Sleep -Seconds 2

& "$PSScriptRoot\target\release\servo-embedding-template-demos.exe" --dev

Stop-Process -Id $vite.Id -ErrorAction SilentlyContinue
