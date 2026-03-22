-- ao_status.lua: AO handshakes/captures and uptime display.
plugin = {}
plugin.name    = "ao_status"
plugin.version = "1.0.0"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

function on_load(config)
    register_indicator("ao_status", {
        x    = config.x,
        y    = config.y,
        font = "small",
    })
end

function on_epoch(state)
    local s = "AO: " .. state.handshakes .. "/" .. state.captures_total
              .. " | " .. state.ao_uptime_str
    set_indicator("ao_status", s)
end
