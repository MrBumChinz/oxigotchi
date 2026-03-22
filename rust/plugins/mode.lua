-- mode.lua: Shows current operating mode (RAGE/SAFE).
plugin = {}
plugin.name    = "mode"
plugin.version = "1.1.0"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

function on_load(config)
    register_indicator("mode", {
        x    = config.x,
        y    = config.y,
        font = "small",
    })
end

function on_epoch(state)
    set_indicator("mode", state.mode)
end
