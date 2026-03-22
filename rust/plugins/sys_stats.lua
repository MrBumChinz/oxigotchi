-- sys_stats.lua: System stats — header line + mem/cpu/freq/temp values.
plugin = {}
plugin.name    = "sys_stats"
plugin.version = "1.0.0"
plugin.author  = "oxigotchi"
plugin.tag     = "default"

function on_load(config)
    -- Header label row
    register_indicator("sys_stats_header", {
        x    = config.x,
        y    = config.y,
        font = "small",
    })
    -- Values row (14px below header to match draw_text line spacing)
    register_indicator("sys_stats_values", {
        x    = config.x,
        y    = config.y + 10,
        font = "small",
    })
end

function on_epoch(state)
    set_indicator("sys_stats_header", "mem  cpu freq temp")

    local mem_pct = "-"
    if state.mem_total_mb > 0 then
        mem_pct = tostring(math.floor(state.mem_used_mb * 100 / state.mem_total_mb))
    end
    local cpu_pct  = tostring(math.floor(state.cpu_percent))
    local freq     = state.cpu_freq_ghz
    local temp     = tostring(math.floor(state.cpu_temp))
    local vals = mem_pct .. "%  " .. cpu_pct .. "% " .. freq .. " " .. temp .. "C"
    set_indicator("sys_stats_values", vals)
end
