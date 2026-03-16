local ScriptWatcher = {}
ScriptWatcher.__index = ScriptWatcher

function ScriptWatcher.new(watchServices, onChange, onCreate, onDelete, onMove)
    local self = setmetatable({}, ScriptWatcher)
    self._connections = {}
    self._tracked = {} -- [Instance] = { path = "..." }
    self._paused = false
    self._watchServices = watchServices
    self._onChange = onChange
    self._onCreate = onCreate
    self._onDelete = onDelete
    self._onMove = onMove
    return self
end

function ScriptWatcher:start()
    for _, serviceName in ipairs(self._watchServices) do
        local ok, service = pcall(function()
            return game:GetService(serviceName)
        end)
        if ok and service then
            self:_scanExisting(service)
            self:_watchDescendants(service)
        end
    end
end

function ScriptWatcher:pause()
    self._paused = true
end

function ScriptWatcher:resume()
    self._paused = false
end

function ScriptWatcher:_isScript(instance)
    return instance:IsA("LuaSourceContainer")
end

function ScriptWatcher:_getDataModelPath(instance)
    local parts = {}
    local current = instance
    while current and current ~= game do
        table.insert(parts, 1, current.Name)
        current = current.Parent
    end
    return table.concat(parts, ".")
end

function ScriptWatcher:_trackInstance(instance)
    if not self:_isScript(instance) then return end

    local path = self:_getDataModelPath(instance)
    self._tracked[instance] = { path = path }

    -- Watch Source changes
    local conn = instance:GetPropertyChangedSignal("Source"):Connect(function()
        if self._paused then return end
        local capturedPath = self._tracked[instance] and self._tracked[instance].path
        if not capturedPath then return end
        -- Debounce
        task.delay(0.2, function()
            if not instance.Parent then return end
            self._onChange(capturedPath, instance.ClassName, instance.Source)
        end)
    end)
    table.insert(self._connections, conn)

    -- Watch Name changes (rename/move)
    local nameConn = instance:GetPropertyChangedSignal("Name"):Connect(function()
        if self._paused then return end
        local info = self._tracked[instance]
        if not info then return end
        local oldPath = info.path
        local newPath = self:_getDataModelPath(instance)
        info.path = newPath
        self._onMove(oldPath, newPath)
    end)
    table.insert(self._connections, nameConn)
end

function ScriptWatcher:_scanExisting(root)
    for _, descendant in ipairs(root:GetDescendants()) do
        self:_trackInstance(descendant)
    end
end

function ScriptWatcher:_watchDescendants(root)
    local addConn = root.DescendantAdded:Connect(function(descendant)
        if self._paused then return end
        if not self:_isScript(descendant) then return end
        task.defer(function()
            if not descendant.Parent then return end
            local path = self:_getDataModelPath(descendant)
            self:_trackInstance(descendant)
            self._onCreate(path, descendant.ClassName, descendant.Source)
        end)
    end)

    local removeConn = root.DescendantRemoving:Connect(function(descendant)
        if self._paused then return end
        if not self._tracked[descendant] then return end
        local path = self._tracked[descendant].path
        self._tracked[descendant] = nil
        self._onDelete(path)
    end)

    table.insert(self._connections, addConn)
    table.insert(self._connections, removeConn)
end

function ScriptWatcher:getAllScripts()
    local scripts = {}
    for instance, info in pairs(self._tracked) do
        table.insert(scripts, {
            datamodel_path = info.path,
            class_name = instance.ClassName,
            source = instance.Source,
        })
    end
    return scripts
end

function ScriptWatcher:findByPath(dmPath)
    for instance, info in pairs(self._tracked) do
        if info.path == dmPath then
            return instance
        end
    end
    return nil
end

function ScriptWatcher:destroy()
    for _, conn in ipairs(self._connections) do
        conn:Disconnect()
    end
    self._connections = {}
    self._tracked = {}
end

return ScriptWatcher
