-- www.lua: Internet connectivity status display.
-- Matches update_display() exactly: WWW:C / WWW:-
-- Note: state.internet_online is true=Online, false=Offline or Unknown.
plugin = {}
plugin.name    = "www"
plugin.version = "1.0.0"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

function on_load(config)
    register_indicator("www", {
        x    = config.x,
        y    = config.y,
        font = "small",
    })
end

function on_epoch(state)
    if state.internet_online then
        set_indicator("www", "WWW:C")
    else
        set_indicator("www", "WWW:-")
    end
end
