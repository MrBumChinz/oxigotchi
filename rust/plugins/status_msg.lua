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
    -- When a joke is active, always show the personality message (question/punchline)
    -- so operational messages ("Scanning...", "Attacking...") don't interrupt jokes.
    if state.joke_active then
        set_indicator("status_msg", state.status_message)
        return
    end
    local msg = state.epoch_phase_status or ""
    if msg == "" or msg == "Sleeping..." then
        msg = state.status_message
    end
    set_indicator("status_msg", msg)
end
