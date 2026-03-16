local UI = {}

local statusLabel = nil
local widget = nil

function UI.init(plugin)
    widget = plugin:CreateDockWidgetPluginGui(
        "ScriptSyncStatus",
        DockWidgetPluginGuiInfo.new(
            Enum.InitialDockState.Bottom,
            false, -- initially disabled
            false, -- override previous state
            200,   -- default width
            100,   -- default height
            150,   -- min width
            80     -- min height
        )
    )
    widget.Title = "ScriptSync"

    local frame = Instance.new("Frame")
    frame.Size = UDim2.new(1, 0, 1, 0)
    frame.BackgroundColor3 = Color3.fromRGB(30, 30, 30)
    frame.BorderSizePixel = 0
    frame.Parent = widget

    statusLabel = Instance.new("TextLabel")
    statusLabel.Size = UDim2.new(1, -20, 0, 30)
    statusLabel.Position = UDim2.new(0, 10, 0, 10)
    statusLabel.BackgroundTransparency = 1
    statusLabel.TextColor3 = Color3.fromRGB(200, 200, 200)
    statusLabel.TextSize = 14
    statusLabel.Font = Enum.Font.SourceSans
    statusLabel.TextXAlignment = Enum.TextXAlignment.Left
    statusLabel.Text = "Not connected"
    statusLabel.Parent = frame
end

function UI.setStatus(text)
    if statusLabel then
        statusLabel.Text = text
    end
end

return UI
