-- ip_display.lua: Rotating USB/BT IP address display.
plugin = {}
plugin.name    = "ip_display"
plugin.version = "1.0.0"
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
