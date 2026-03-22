-- status_msg.lua: Bull-themed personality status message, word-wrapped.
plugin = {}
plugin.name    = "status_msg"
plugin.version = "1.0.0"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

function on_load(config)
    register_indicator("status_msg", {
        x          = config.x,
        y          = config.y,
        font       = "medium",
        wrap_width = 17,
    })
end

function on_epoch(state)
    set_indicator("status_msg", state.status_message)
end
