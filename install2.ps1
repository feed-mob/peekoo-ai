$pluginPath = "$env:LOCALAPPDATA\Peekoo\peekoo\plugins\pomodoro\target\wasm32-wasip1\release"
if (-Not (Test-Path $pluginPath)) {
    New-Item -ItemType Directory -Force -Path $pluginPath | Out-Null
}
Copy-Item "plugins\pomodoro\target\wasm32-wasip1\release\pomodoro.wasm" "$pluginPath\pomodoro.wasm" -Force
Copy-Item "plugins\pomodoro\peekoo-plugin.toml" "$env:LOCALAPPDATA\Peekoo\peekoo\plugins\pomodoro\peekoo-plugin.toml" -Force
Write-Output "Plugin installed to $env:LOCALAPPDATA\Peekoo\peekoo\plugins\pomodoro"
