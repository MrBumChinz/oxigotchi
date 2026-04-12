-- ip_display.lua: USB IP address display.
-- Shows the USB gadget IP (state.display_ip). BT IP is handled
-- separately by bt_ip_display.lua on the line above.
plugin = {}
plugin.name    = "ip_display"
plugin.version = "3.3.3"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

function on_load(config)
    register_indicator("ip_display", {
        x    = config.x,
        y    = config.y,
        font = "small",
    })
end

function on_epoch(state)
    set_indicator("ip_display", state.display_ip)
end
