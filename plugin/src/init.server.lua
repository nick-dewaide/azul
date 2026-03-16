--[[
    ScriptSync Plugin
    Bidirectional script synchronization between Roblox Studio and the filesystem.
]]

local HttpService = game:GetService("HttpService")
local ScriptWatcher = require(script.ScriptWatcher)
local SyncClient = require(script.SyncClient)
local Config = require(script.Config)
local UI = require(script.UI)

local toolbar = plugin:CreateToolbar("ScriptSync")
local connectButton = toolbar:CreateButton(
    "Connect",
    "Connect to ScriptSync server",
    "rbxassetid://0"
)

local state = {
    connected = false,
    client = nil,
    watcher = nil,
}

local function disconnect()
    if state.client then
        state.client:disconnect()
        state.client = nil
    end
    if state.watcher then
        state.watcher:destroy()
        state.watcher = nil
    end
    state.connected = false
    UI.setStatus("Disconnected")
end

local function connect()
    if state.connected then
        disconnect()
        return
    end

    UI.setStatus("Connecting...")

    local watcher = ScriptWatcher.new(
        Config.watchServices,
        function(dmPath, className, source)
            -- onChange
            if state.client then
                state.client:sendSourceChanged(dmPath, className, source)
            end
        end,
        function(dmPath, className, source)
            -- onCreate
            if state.client then
                state.client:sendScriptCreated(dmPath, className, source)
            end
        end,
        function(dmPath)
            -- onDelete
            if state.client then
                state.client:sendScriptDeleted(dmPath)
            end
        end,
        function(oldPath, newPath)
            -- onMove
            if state.client then
                state.client:sendScriptMoved(oldPath, newPath)
            end
        end
    )
    watcher:start()
    state.watcher = watcher

    local client = SyncClient.new(Config.host, Config.port, watcher)

    local success, err = pcall(function()
        client:connect()
    end)

    if success then
        state.client = client
        state.connected = true
        UI.setStatus("Connected")
    else
        watcher:destroy()
        state.watcher = nil
        UI.setStatus("Connection failed: " .. tostring(err))
        warn("[ScriptSync] Connection failed: " .. tostring(err))
    end
end

connectButton.Click:Connect(connect)

plugin.Unloading:Connect(function()
    disconnect()
end)

UI.init(plugin)
UI.setStatus("Not connected")
