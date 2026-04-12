-- ip_display.lua: IP address display with BT/USB rotation.
-- When BT is tethered, alternates between BT IP and USB IP every 5 epochs.
-- When BT is not connected, shows USB IP only.
plugin = {}
plugin.name    = "ip_display"
plugin.version = "3.3.2"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

local tick = 0

function on_load(config)
    register_indicator("ip_display", {
        x    = config.x,
        y    = config.y,
        font = "small",
    })
end

function on_epoch(state)
    tick = tick + 1
    local bt_ip = state.bt_ip or ""
    local usb_ip = state.display_ip or ""

    if bt_ip ~= "" then
        -- BT tethered: rotate every 5 epochs
        if (math.floor((tick - 1) / 5) % 2) == 0 then
            set_indicator("ip_display", "BT " .. bt_ip)
        else
            set_indicator("ip_display", usb_ip)
        end
    else
        set_indicator("ip_display", usb_ip)
    end
end
