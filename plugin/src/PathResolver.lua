local PathResolver = {}

function PathResolver.resolve(datamodelPath)
    local parts = string.split(datamodelPath, ".")
    local scriptName = table.remove(parts)

    local current = game
    for _, part in ipairs(parts) do
        local child = current:FindFirstChild(part)
        if not child then
            return nil, "Parent not found: " .. part
        end
        current = child
    end

    return current, scriptName
end

function PathResolver.createScript(datamodelPath, className, source)
    local parent, name = PathResolver.resolve(datamodelPath)
    if not parent then
        return nil, name -- name contains error message
    end

    local script = Instance.new(className)
    script.Name = name
    script.Source = source
    script.Parent = parent
    return script
end

return PathResolver
