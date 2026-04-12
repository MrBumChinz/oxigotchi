-- bt_ip_display.lua: BT tether IP address display.
-- Shows "BT <ip>" when phone tether is connected, blank otherwise.
-- Positioned above ip_display (USB IP) so both are visible at once.
plugin = {}
plugin.name    = "bt_ip_display"
plugin.version = "3.3.3"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

function on_load(config)
    register_indicator("bt_ip_display", {
        x    = config.x,
        y    = config.y,
        font = "small",
    })
end

function on_epoch(state)
    local bt_ip = state.bt_ip or ""
    if bt_ip ~= "" then
        set_indicator("bt_ip_display", "BT " .. bt_ip)
    else
        set_indicator("bt_ip_display", "")
    end
end
