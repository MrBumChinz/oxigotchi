-- battery.lua: Battery level and charge state display.
-- Mirrors pisugar::PiSugar::display_str(): "CHG=N%" when charging/full,
-- "BAT=N%" when discharging, "BAT N/A" when not available.
plugin = {}
plugin.name    = "battery"
plugin.version = "1.0.0"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

function on_load(config)
    register_indicator("battery", {
        x    = config.x,
        y    = config.y,
        font = "small",
    })
end

function on_epoch(state)
    local s
    if not state.battery_available then
        s = "BAT N/A"
    elseif state.battery_charging then
        s = "CHG=" .. state.battery_level .. "%"
    else
        s = "BAT=" .. state.battery_level .. "%"
    end
    set_indicator("battery", s)
end
