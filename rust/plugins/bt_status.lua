-- bt_status.lua: Bluetooth tether status display.
plugin = {}
plugin.name    = "bt_status"
plugin.version = "1.0.0"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

function on_load(config)
    register_indicator("bt_status", {
        x    = config.x,
        y    = config.y,
        font = "small",
    })
end

function on_epoch(state)
    set_indicator("bt_status", "BT:" .. state.bt_short)
end
