-- uptime.lua: System uptime with "UP: HH:MM:SS" label.
plugin = {}
plugin.name    = "uptime"
plugin.version = "1.0.0"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

function on_load(config)
    register_indicator("uptime", {
        x     = config.x,
        y     = config.y,
        font  = "small",
        label = "UP",
    })
end

function on_epoch(state)
    set_indicator("uptime", format_duration(state.uptime_secs))
end
