local HttpService = game:GetService("HttpService")

local SyncClient = {}
SyncClient.__index = SyncClient

function SyncClient.new(host, port, scriptWatcher)
    local self = setmetatable({}, SyncClient)
    self._host = host
    self._port = port
    self._scriptWatcher = scriptWatcher
    self._connected = false
    self._polling = false
    return self
end

function SyncClient:connect()
    -- Test connection with server info endpoint
    local url = string.format("http://%s:%d/api/scriptsync", self._host, self._port)
    local response = HttpService:RequestAsync({
        Url = url,
        Method = "GET",
    })

    if not response.Success then
        error("Failed to connect to ScriptSync server at " .. url)
    end

    local serverInfo = HttpService:JSONDecode(response.Body)
    print("[ScriptSync] Connected to server v" .. (serverInfo.server_version or "unknown"))

    self._connected = true

    -- Send initial sync
    local allScripts = self._scriptWatcher:getAllScripts()
    local responses = self:_sendSync({
        type = "initial_sync",
        scripts = allScripts,
    })

    -- Process immediate responses (like sync_confirmed)
    if responses then
        for _, msg in ipairs(responses) do
            self:_handleMessage(msg)
        end
    end

    -- Start polling for server→plugin updates
    self:_startPolling()
end

function SyncClient:disconnect()
    self._connected = false
    self._polling = false
end

function SyncClient:sendSourceChanged(dmPath, className, source)
    if not self._connected then return end
    self:_sendSync({
        type = "source_changed",
        datamodel_path = dmPath,
        class_name = className,
        source = source,
        checksum = self:_checksum(source),
    })
end

function SyncClient:sendScriptCreated(dmPath, className, source)
    if not self._connected then return end
    self:_sendSync({
        type = "script_created",
        datamodel_path = dmPath,
        class_name = className,
        source = source,
    })
end

function SyncClient:sendScriptDeleted(dmPath)
    if not self._connected then return end
    self:_sendSync({
        type = "script_deleted",
        datamodel_path = dmPath,
    })
end

function SyncClient:sendScriptMoved(oldPath, newPath)
    if not self._connected then return end
    self:_sendSync({
        type = "script_moved",
        old_datamodel_path = oldPath,
        new_datamodel_path = newPath,
    })
end

-- Send a message to the server via HTTP POST and return parsed responses.
function SyncClient:_sendSync(data)
    local url = string.format("http://%s:%d/api/sync", self._host, self._port)
    local body = HttpService:JSONEncode(data)

    local ok, response = pcall(function()
        return HttpService:RequestAsync({
            Url = url,
            Method = "POST",
            Headers = {
                ["Content-Type"] = "application/json",
            },
            Body = body,
        })
    end)

    if not ok then
        warn("[ScriptSync] Failed to send " .. (data.type or "unknown") .. ": " .. tostring(response))
        return nil
    end

    if response.Success then
        local success, decoded = pcall(HttpService.JSONDecode, HttpService, response.Body)
        if success then
            return decoded
        end
    else
        warn("[ScriptSync] Server returned " .. tostring(response.StatusCode) .. " for " .. (data.type or "unknown"))
    end

    return nil
end

function SyncClient:_startPolling()
    self._polling = true

    task.spawn(function()
        while self._connected and self._polling do
            local ok, err = pcall(function()
                self:_pollForUpdates()
            end)
            if not ok then
                warn("[ScriptSync] Poll error: " .. tostring(err))
            end
            task.wait(0.5)
        end
    end)
end

function SyncClient:_pollForUpdates()
    local url = string.format("http://%s:%d/api/poll", self._host, self._port)

    local ok, response = pcall(function()
        return HttpService:RequestAsync({
            Url = url,
            Method = "GET",
        })
    end)

    if not ok then
        return
    end

    if not response.Success then
        return
    end

    local success, messages = pcall(HttpService.JSONDecode, HttpService, response.Body)
    if not success or type(messages) ~= "table" then
        return
    end

    for _, msg in ipairs(messages) do
        self:_handleMessage(msg)
    end
end

function SyncClient:_handleMessage(msg)
    if msg.type == "update_source" then
        local instance = self._scriptWatcher:findByPath(msg.datamodel_path)
        if instance then
            self._scriptWatcher:pause()
            instance.Source = msg.source
            task.defer(function()
                self._scriptWatcher:resume()
            end)
        end
    elseif msg.type == "create_script" then
        self._scriptWatcher:pause()
        local PathResolver = require(script.Parent.PathResolver)
        local ok, result = pcall(function()
            return PathResolver.createScript(msg.datamodel_path, msg.class_name, msg.source)
        end)
        task.defer(function()
            self._scriptWatcher:resume()
        end)
        if not ok then
            warn("[ScriptSync] Failed to create script: " .. tostring(result))
        end
    elseif msg.type == "file_deleted" then
        warn("[ScriptSync] File deleted from disk: " .. msg.datamodel_path)
    elseif msg.type == "sync_confirmed" then
        print("[ScriptSync] Sync confirmed: " .. tostring(msg.count) .. " scripts")
    end
end

function SyncClient:_checksum(source)
    local hash = 0
    for i = 1, math.min(#source, 100) do
        hash = (hash * 31 + string.byte(source, i)) % 2147483647
    end
    return "hash:" .. tostring(#source) .. ":" .. tostring(hash)
end

return SyncClient
