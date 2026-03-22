-- mode.lua: Static operating mode label ("AUTO").
plugin = {}
plugin.name    = "mode"
plugin.version = "1.0.0"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

function on_load(config)
    register_indicator("mode", {
        x    = config.x,
        y    = config.y,
        font = "small",
    })
    -- Static value set at load time; no state needed.
    set_indicator("mode", "AUTO")
end

function on_epoch(state)
    -- Mode is always AUTO in AO mode; no update needed.
end
