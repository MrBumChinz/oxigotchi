//! Dashboard HTML template — embedded single-page web UI.

/// The full dashboard HTML/CSS/JS served at GET /.
pub const DASHBOARD_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no">
<title>oxigotchi</title>
<script src="https://unpkg.com/htmx.org@1.9.10"></script>
<style>
*{box-sizing:border-box;margin:0;padding:0}
body{background:#1a1a2e;color:#e0e0e0;font-family:'SF Mono','Fira Code','Cascadia Code',monospace;font-size:14px;padding:12px;max-width:600px;margin:0 auto}
h1{color:#00d4aa;font-size:20px;text-align:center;margin-bottom:16px;letter-spacing:1px}
.card{background:#16213e;border-radius:12px;padding:16px;margin-bottom:12px}
.card-title{color:#00d4aa;font-size:15px;font-weight:bold;margin-bottom:12px;padding-bottom:8px;border-bottom:1px solid #0f3460}
.face{font-size:48px;text-align:center;padding:20px;color:#e0e0e0}
.status-grid{display:grid;grid-template-columns:1fr 1fr;gap:6px 16px}
.status-grid .label{color:#888;font-size:12px}
.status-grid .value{color:#e0e0e0;font-size:13px;font-weight:bold}
.stat-row{display:flex;flex-wrap:wrap;gap:8px}
.stat{text-align:center;flex:1;min-width:60px}
.stat .label{color:#888;font-size:11px}
.stat .value{color:#00d4aa;font-size:18px;font-weight:bold}
.health-row{display:flex;flex-wrap:wrap;gap:10px;margin-bottom:4px}
.health-item{display:flex;align-items:center;gap:6px;font-size:13px}
.dot{width:10px;height:10px;border-radius:50%;display:inline-block}
.dot-green{background:#00d4aa}
.dot-red{background:#e94560}
.dot-gray{background:#555}
.dot-yellow{background:#f0c040}
.toggle-row{display:flex;align-items:center;justify-content:space-between;padding:10px 0;border-bottom:1px solid #0f3460}
.toggle-row:last-child{border-bottom:none}
.toggle-info{flex:1;margin-right:12px}
.toggle-label{font-size:14px;font-weight:bold;color:#e0e0e0}
.toggle-desc{font-size:11px;color:#888;margin-top:2px}
.switch{position:relative;width:50px;height:28px;flex-shrink:0}
.switch input{opacity:0;width:0;height:0}
.slider{position:absolute;cursor:pointer;top:0;left:0;right:0;bottom:0;background:#555;border-radius:28px;transition:.25s}
.slider:before{position:absolute;content:"";height:22px;width:22px;left:3px;bottom:3px;background:#fff;border-radius:50%;transition:.25s}
input:checked+.slider{background:#00d4aa}
input:checked+.slider:before{transform:translateX(22px)}
.rate-btns{display:flex;gap:8px;margin-top:8px}
.rate-btn{flex:1;padding:14px 0;border:2px solid #0f3460;border-radius:10px;background:transparent;color:#e0e0e0;font-size:18px;font-weight:bold;font-family:inherit;cursor:pointer;text-align:center;transition:.2s}
.rate-btn.active{background:#0f3460;color:#00d4aa;border-color:#00d4aa}
.rate-btn.risky{border-color:#e67e22;color:#e67e22}
.rate-btn.risky.active{background:#5a3000;color:#e67e22;border-color:#e67e22}
.rate-btn:active{transform:scale(0.95)}
.mode-btns{display:flex;gap:8px;margin-top:8px}
.mode-btn{flex:1;padding:14px 0;border:2px solid #0f3460;border-radius:10px;background:transparent;color:#e0e0e0;font-size:16px;font-weight:bold;font-family:inherit;cursor:pointer;text-align:center;transition:.2s}
.mode-btn.active{background:#00d4aa;color:#1a1a2e;border-color:#00d4aa}
.mode-btn:active{transform:scale(0.95)}
.action-btns{display:flex;flex-wrap:wrap;gap:8px}
.action-btn{flex:1;min-width:100px;padding:14px 8px;border:none;border-radius:10px;font-family:inherit;font-size:13px;font-weight:bold;cursor:pointer;text-align:center;transition:.2s}
.action-btn:active{transform:scale(0.95)}
.btn-restart{background:#0f3460;color:#00d4aa}
.btn-stop{background:#e94560;color:#fff}
.btn-warn{background:#f0c040;color:#1a1a2e}
.captures-list{max-height:200px;overflow-y:auto;margin-top:8px}
.capture-item{font-size:12px;color:#aaa;padding:4px 0;border-bottom:1px solid #0f346033}
.capture-item:last-child{border-bottom:none}
.toast{position:fixed;bottom:20px;left:50%;transform:translateX(-50%);background:#00d4aa;color:#1a1a2e;padding:10px 20px;border-radius:8px;font-size:13px;font-weight:bold;opacity:0;transition:opacity .3s;pointer-events:none;z-index:999}
.toast.show{opacity:1}
.progress-bar{height:6px;background:#0f3460;border-radius:3px;overflow:hidden;margin-top:4px}
.progress-fill{height:100%;background:#00d4aa;border-radius:3px;transition:width .3s}
.grid-2{display:grid;grid-template-columns:1fr 1fr;gap:8px}
.sub{color:#888;font-size:11px;margin-bottom:8px}
</style>
</head>
<body>
<h1>Oxigotchi Dashboard</h1>
<div style="text-align:center;color:#888;font-size:11px;margin:-12px 0 14px">Rusty Oxigotchi &mdash; WiFi capture bull</div>

<!-- 1. Face display -->
<div class="card" id="card-face">
<div class="face" id="face">(O_O)</div>
<div style="text-align:center;color:#888" id="status-msg">Loading...</div>
</div>

<!-- 2. Core stats -->
<div class="card" id="card-stats">
<div class="card-title">Core Stats</div>
<div class="stat-row">
<div class="stat"><div class="label">CH</div><div class="value" id="s-ch">-</div></div>
<div class="stat"><div class="label">APS</div><div class="value" id="s-aps">-</div></div>
<div class="stat"><div class="label">PWND</div><div class="value" id="s-pwnd">-</div></div>
<div class="stat"><div class="label">EPOCH</div><div class="value" id="s-epoch">-</div></div>
<div class="stat"><div class="label">UPTIME</div><div class="value" id="s-uptime">-</div></div>
<div class="stat"><div class="label">RATE</div><div class="value" id="s-rate">-</div></div>
</div>
</div>

<!-- 3. E-ink preview -->
<div class="card" id="card-eink" style="text-align:center">
<div class="card-title">Live Display</div>
<div style="padding:8px;background:#fff;display:inline-block;border-radius:4px"><img id="eink-img" src="/api/display.png" alt="e-ink" style="width:250px;height:122px;image-rendering:pixelated"></div>
</div>

<div class="grid-2">

<!-- 4. Battery -->
<div class="card" id="card-battery">
<div class="card-title">Battery</div>
<div class="status-grid">
<div class="label">Level</div><div class="value" id="bat-level">-</div>
<div class="label">State</div><div class="value" id="bat-state">-</div>
<div class="label">Voltage</div><div class="value" id="bat-voltage">-</div>
</div>
<div class="progress-bar"><div class="progress-fill" id="bat-bar" style="width:0%"></div></div>
</div>

<!-- 5. Bluetooth -->
<div class="card" id="card-bt">
<div class="card-title">Bluetooth</div>
<div class="status-grid">
<div class="label">Status</div><div class="value" id="bt-status">-</div>
<div class="label">Device</div><div class="value" id="bt-device">-</div>
<div class="label">IP</div><div class="value" id="bt-ip">-</div>
</div>
</div>

</div>

<!-- 6. WiFi -->
<div class="card" id="card-wifi">
<div class="card-title">WiFi</div>
<div class="sub">Monitor mode status and channel info.</div>
<div class="status-grid">
<div class="label">State</div><div class="value" id="wifi-state">-</div>
<div class="label">Channel</div><div class="value" id="wifi-ch">-</div>
<div class="label">APs Tracked</div><div class="value" id="wifi-aps">-</div>
<div class="label">Channels</div><div class="value" id="wifi-channels">-</div>
<div class="label">Dwell</div><div class="value" id="wifi-dwell">-</div>
</div>
</div>

<!-- 7. Attack controls -->
<div class="card" id="card-attacks">
<div class="card-title">Attack Types</div>
<div style="color:#00d4aa;font-size:11px;margin-bottom:10px;padding:8px;background:#0f346033;border-radius:6px">All 6 ON is the sweet spot &mdash; they complement each other.</div>
<div class="toggle-row">
<div class="toggle-info"><div class="toggle-label">Deauth</div><div class="toggle-desc">Kick clients to capture reconnection handshakes</div></div>
<label class="switch"><input type="checkbox" id="atk-deauth" checked onchange="toggleAttack('deauth',this.checked)"><span class="slider"></span></label>
</div>
<div class="toggle-row">
<div class="toggle-info"><div class="toggle-label">PMKID</div><div class="toggle-desc">Grab router password hashes without clients</div></div>
<label class="switch"><input type="checkbox" id="atk-pmkid" checked onchange="toggleAttack('pmkid',this.checked)"><span class="slider"></span></label>
</div>
<div class="toggle-row">
<div class="toggle-info"><div class="toggle-label">CSA</div><div class="toggle-desc">Trick clients into switching channels</div></div>
<label class="switch"><input type="checkbox" id="atk-csa" checked onchange="toggleAttack('csa',this.checked)"><span class="slider"></span></label>
</div>
<div class="toggle-row">
<div class="toggle-info"><div class="toggle-label">Disassociation</div><div class="toggle-desc">Catches clients that resist deauth</div></div>
<label class="switch"><input type="checkbox" id="atk-disassoc" checked onchange="toggleAttack('disassoc',this.checked)"><span class="slider"></span></label>
</div>
<div class="toggle-row">
<div class="toggle-info"><div class="toggle-label">Anon Reassoc</div><div class="toggle-desc">Capture PMKID from stubborn routers</div></div>
<label class="switch"><input type="checkbox" id="atk-anon_reassoc" checked onchange="toggleAttack('anon_reassoc',this.checked)"><span class="slider"></span></label>
</div>
<div class="toggle-row">
<div class="toggle-info"><div class="toggle-label">Rogue M2</div><div class="toggle-desc">Fake AP trick for handshakes</div></div>
<label class="switch"><input type="checkbox" id="atk-rogue_m2" checked onchange="toggleAttack('rogue_m2',this.checked)"><span class="slider"></span></label>
</div>

<div style="margin-top:12px;padding-top:10px;border-top:1px solid #0f3460">
<div style="font-size:12px;color:#888;margin-bottom:4px">Attack Rate</div>
<div class="sub">Rate 1 is max safe for BCM43436B0. Higher rates cause firmware crashes.</div>
<div class="rate-btns">
<button class="rate-btn active" id="rate-1" onclick="setRate(1)">1<br><span style="font-size:10px;font-weight:normal;color:#888">Safe</span></button>
<button class="rate-btn risky" id="rate-2" onclick="setRate(2)">2<br><span style="font-size:10px;font-weight:normal">Risky</span></button>
<button class="rate-btn risky" id="rate-3" onclick="setRate(3)">3<br><span style="font-size:10px;font-weight:normal">Danger</span></button>
</div>
</div>
</div>

<!-- 8. Capture list -->
<div class="card" id="card-captures">
<div class="card-title">Recent Captures</div>
<div class="sub">Validated capture files. Click to download.</div>
<div class="status-grid" style="margin-bottom:8px">
<div class="label">Total Files</div><div class="value" id="cap-total">-</div>
<div class="label">Handshakes</div><div class="value" id="cap-hs">-</div>
<div class="label">Pending Upload</div><div class="value" id="cap-pending">-</div>
<div class="label">Total Size</div><div class="value" id="cap-size">-</div>
</div>
<div class="captures-list" id="cap-list"><div style="color:#555;font-size:12px">Loading...</div></div>
</div>

<!-- 9. Recovery status -->
<div class="card" id="card-recovery">
<div class="card-title">Recovery Status</div>
<div class="sub">WiFi and firmware crash recovery tracking.</div>
<div class="health-row" style="margin-bottom:8px">
<div class="health-item"><span class="dot dot-gray" id="h-wifi"></span>WiFi</div>
<div class="health-item"><span class="dot dot-gray" id="h-ao"></span>AO</div>
<div class="health-item"><span class="dot dot-gray" id="h-recovery"></span>Recovery</div>
</div>
<div class="status-grid">
<div class="label">State</div><div class="value" id="rec-state">-</div>
<div class="label">Crashes</div><div class="value" id="rec-crashes">-</div>
<div class="label">Recoveries</div><div class="value" id="rec-total">-</div>
<div class="label">Last Recovery</div><div class="value" id="rec-last">-</div>
<div class="label">AO PID</div><div class="value" id="rec-pid">-</div>
<div class="label">AO Uptime</div><div class="value" id="rec-ao-up">-</div>
</div>
</div>

<!-- 10. Personality -->
<div class="card" id="card-personality">
<div class="card-title">Personality</div>
<div class="sub">Mood, experience, and level progression.</div>
<div class="status-grid">
<div class="label">Mood</div><div class="value" id="p-mood">-</div>
<div class="label">Face</div><div class="value" id="p-face">-</div>
<div class="label">XP</div><div class="value" id="p-xp">-</div>
<div class="label">Level</div><div class="value" id="p-level">-</div>
<div class="label">Blind Epochs</div><div class="value" id="p-blind">-</div>
</div>
<div class="progress-bar" style="margin-top:8px"><div class="progress-fill" id="mood-bar" style="width:50%"></div></div>
</div>

<!-- 11. System info -->
<div class="card" id="card-system">
<div class="card-title">System Info</div>
<div class="sub">Hardware stats from the Pi.</div>
<div class="status-grid">
<div class="label">CPU Temp</div><div class="value" id="sys-temp">-</div>
<div class="label">CPU Usage</div><div class="value" id="sys-cpu">-</div>
<div class="label">Memory</div><div class="value" id="sys-mem">-</div>
<div class="label">Disk</div><div class="value" id="sys-disk">-</div>
<div class="label">Sys Uptime</div><div class="value" id="sys-uptime">-</div>
</div>
</div>

<!-- 12. Cracked passwords -->
<div class="card" id="card-cracked">
<div class="card-title">Cracked Passwords</div>
<div class="sub">Passwords cracked from captured handshakes.</div>
<div id="cracked-list"><div style="color:#555;font-size:12px">No cracked passwords yet</div></div>
</div>

<!-- 13. Handshake download -->
<div class="card" id="card-download">
<div class="card-title">Download Captures</div>
<div class="sub">Download all captures as a ZIP archive.</div>
<div class="action-btns">
<a href="/api/download/all" class="action-btn btn-restart" style="text-decoration:none;text-align:center">Download All (ZIP)</a>
</div>
</div>

<!-- 14. Mode switch -->
<div class="card" id="card-mode">
<div class="card-title">Mode</div>
<div class="sub">AO Mode = AngryOxide attacks. PWN Mode = stock bettercap. Switching takes ~90s.</div>
<div class="mode-btns">
<button class="mode-btn active" id="mode-ao" onclick="switchMode('AO')">AO Mode</button>
<button class="mode-btn" id="mode-pwn" onclick="switchMode('PWN')">PWN Mode</button>
</div>
</div>

<!-- 15. Actions -->
<div class="card" id="card-actions">
<div class="card-title">Actions</div>
<div class="sub">Restart applies config changes. Shutdown powers off the Pi.</div>
<div class="action-btns">
<button class="action-btn btn-restart" onclick="restartAO()">Restart AO</button>
<button class="action-btn btn-stop" onclick="if(confirm('Shut down the Pi?'))doShutdown()">Shutdown Pi</button>
<button class="action-btn btn-warn" onclick="if(confirm('Restart pwnagotchi?'))restartPwn()">Restart Pwn</button>
</div>
</div>

<!-- 16. Plugins -->
<div class="card" id="card-plugins">
<div class="card-title">Plugins</div>
<div class="sub">Lua plugins control display indicators. Toggle on/off and set x,y positions.</div>
<div id="plugins-list"><div style="color:#555;font-size:12px">Loading...</div></div>
</div>

<div style="text-align:center;color:#555;font-size:10px;margin-top:8px">Auto-refreshes every 5s &bull; Rusty Oxigotchi</div>

<div class="toast" id="toast"></div>

<script>
function api(method, path, body) {
    var opts = {method: method, headers: {'Content-Type':'application/json'}};
    if (body) opts.body = JSON.stringify(body);
    return fetch(path, opts).then(function(r){return r.json()}).catch(function(e){console.error('API:',path,e)});
}
function toast(msg) {
    var t = document.getElementById('toast');
    t.textContent = msg;
    t.classList.add('show');
    setTimeout(function(){t.classList.remove('show')}, 1500);
}
function fmtUptime(secs) {
    if (!secs && secs !== 0) return '--';
    var h = Math.floor(secs/3600), m = Math.floor((secs%3600)/60), s = secs%60;
    return String(h).padStart(2,'0')+':'+String(m).padStart(2,'0')+':'+String(s).padStart(2,'0');
}
function fmtBytes(b) {
    if (b < 1024) return b + ' B';
    if (b < 1048576) return (b/1024).toFixed(1) + ' KB';
    return (b/1048576).toFixed(1) + ' MB';
}
function esc(s) { var d = document.createElement('div'); d.textContent = s; return d.innerHTML; }

// --- Refresh functions ---

function refreshStatus() {
    api('GET', '/api/status').then(function(d) {
        if (!d) return;
        document.getElementById('face').textContent = d.face;
        document.getElementById('status-msg').textContent = d.status_message;
        document.getElementById('s-ch').textContent = d.channel;
        document.getElementById('s-aps').textContent = d.aps_seen;
        document.getElementById('s-pwnd').textContent = d.handshakes;
        document.getElementById('s-epoch').textContent = d.epoch;
        document.getElementById('s-uptime').textContent = d.uptime;
        // Mode buttons
        document.getElementById('mode-ao').classList.toggle('active', d.mode === 'AO');
        document.getElementById('mode-pwn').classList.toggle('active', d.mode === 'PWN');
    });
}

function refreshBattery() {
    api('GET', '/api/battery').then(function(d) {
        if (!d) return;
        if (d.available) {
            document.getElementById('bat-level').textContent = d.level + '%';
            document.getElementById('bat-level').style.color = d.critical ? '#e94560' : (d.low ? '#f0c040' : '#00d4aa');
            document.getElementById('bat-state').textContent = d.charging ? 'Charging' : 'Discharging';
            document.getElementById('bat-voltage').textContent = (d.voltage_mv / 1000).toFixed(2) + 'V';
            document.getElementById('bat-bar').style.width = d.level + '%';
            document.getElementById('bat-bar').style.background = d.critical ? '#e94560' : (d.low ? '#f0c040' : '#00d4aa');
        } else {
            document.getElementById('bat-level').textContent = 'N/A';
            document.getElementById('bat-state').textContent = 'Not detected';
            document.getElementById('bat-voltage').textContent = '-';
        }
    });
}

function refreshBluetooth() {
    api('GET', '/api/bluetooth').then(function(d) {
        if (!d) return;
        document.getElementById('bt-status').textContent = d.connected ? 'Connected' : d.state;
        document.getElementById('bt-status').style.color = d.connected ? '#00d4aa' : '#888';
        document.getElementById('bt-device').textContent = d.device_name || '-';
        document.getElementById('bt-ip').textContent = d.ip || '-';
    });
}

function refreshWifi() {
    api('GET', '/api/wifi').then(function(d) {
        if (!d) return;
        document.getElementById('wifi-state').textContent = d.state;
        document.getElementById('wifi-state').style.color = d.state === 'Monitor' ? '#00d4aa' : '#e94560';
        document.getElementById('wifi-ch').textContent = d.channel;
        document.getElementById('wifi-aps').textContent = d.aps_tracked;
        document.getElementById('wifi-channels').textContent = d.channels.join(', ') || '-';
        document.getElementById('wifi-dwell').textContent = d.dwell_ms + 'ms';
    });
}

function refreshAttacks() {
    api('GET', '/api/attacks').then(function(d) {
        if (!d) return;
        document.getElementById('s-rate').textContent = d.attack_rate;
        ['deauth','pmkid','csa','disassoc','anon_reassoc','rogue_m2'].forEach(function(k) {
            var cb = document.getElementById('atk-'+k);
            if (cb) cb.checked = d[k];
        });
        [1,2,3].forEach(function(n) {
            document.getElementById('rate-'+n).classList.toggle('active', n === d.attack_rate);
        });
    });
}

function refreshCaptures() {
    api('GET', '/api/captures').then(function(d) {
        if (!d) return;
        document.getElementById('cap-total').textContent = d.total_files;
        document.getElementById('cap-hs').textContent = d.handshake_files;
        document.getElementById('cap-pending').textContent = d.pending_upload;
        document.getElementById('cap-size').textContent = fmtBytes(d.total_size_bytes);
        var el = document.getElementById('cap-list');
        if (!d.files || !d.files.length) {
            el.innerHTML = '<div style="color:#555;font-size:12px">No captures yet</div>';
            return;
        }
        el.innerHTML = d.files.map(function(f) {
            return '<div class="capture-item">' + esc(f.filename) + ' <span style="color:#555">(' + fmtBytes(f.size_bytes) + ')</span></div>';
        }).join('');
    });
}

function refreshRecovery() {
    api('GET', '/api/recovery').then(function(d) {
        if (!d) return;
        document.getElementById('rec-state').textContent = d.state;
        document.getElementById('rec-state').style.color = d.state === 'Healthy' ? '#00d4aa' : '#f0c040';
        document.getElementById('rec-total').textContent = d.total_recoveries;
        document.getElementById('rec-last').textContent = d.last_recovery;
    });
    api('GET', '/api/health').then(function(d) {
        if (!d) return;
        document.getElementById('rec-crashes').textContent = d.ao_crash_count;
        document.getElementById('rec-crashes').style.color = d.ao_crash_count > 0 ? '#f0c040' : '#e0e0e0';
        document.getElementById('rec-pid').textContent = d.ao_pid || '-';
        document.getElementById('rec-ao-up').textContent = d.ao_uptime;
        // Health dots
        var wdot = document.getElementById('h-wifi');
        wdot.className = 'dot ' + (d.wifi_state === 'Monitor' ? 'dot-green' : 'dot-red');
        var adot = document.getElementById('h-ao');
        adot.className = 'dot ' + (d.ao_state === 'RUNNING' ? 'dot-green' : 'dot-red');
        var rdot = document.getElementById('h-recovery');
        rdot.className = 'dot ' + (d.ao_crash_count === 0 ? 'dot-green' : 'dot-yellow');
        document.getElementById('sys-uptime').textContent = fmtUptime(d.uptime_secs);
    });
}

function refreshPersonality() {
    api('GET', '/api/personality').then(function(d) {
        if (!d) return;
        document.getElementById('p-mood').textContent = Math.round(d.mood * 100) + '%';
        document.getElementById('p-face').textContent = d.face;
        document.getElementById('p-xp').textContent = d.xp;
        document.getElementById('p-level').textContent = d.level;
        document.getElementById('p-blind').textContent = d.blind_epochs;
        document.getElementById('mood-bar').style.width = Math.round(d.mood * 100) + '%';
        var moodColor = d.mood > 0.7 ? '#00d4aa' : (d.mood > 0.3 ? '#f0c040' : '#e94560');
        document.getElementById('mood-bar').style.background = moodColor;
    });
}

function refreshSystem() {
    api('GET', '/api/system').then(function(d) {
        if (!d) return;
        document.getElementById('sys-temp').textContent = d.cpu_temp_c > 0 ? d.cpu_temp_c.toFixed(1) + '\u00B0C' : '-';
        document.getElementById('sys-temp').style.color = d.cpu_temp_c > 70 ? '#e94560' : (d.cpu_temp_c > 55 ? '#f0c040' : '#00d4aa');
        document.getElementById('sys-cpu').textContent = d.cpu_percent > 0 ? d.cpu_percent.toFixed(0) + '%' : '-';
        document.getElementById('sys-mem').textContent = d.mem_total_mb > 0 ? d.mem_used_mb + '/' + d.mem_total_mb + ' MB' : '-';
        document.getElementById('sys-disk').textContent = d.disk_total_mb > 0 ? d.disk_used_mb + '/' + d.disk_total_mb + ' MB' : '-';
    });
}

function refreshCracked() {
    api('GET', '/api/cracked').then(function(list) {
        var el = document.getElementById('cracked-list');
        if (!list || !list.length) {
            el.innerHTML = '<div style="color:#555;font-size:12px">No cracked passwords yet</div>';
            return;
        }
        el.innerHTML = list.map(function(c) {
            return '<div style="padding:4px 0;border-bottom:1px solid #0f346022">' +
                '<span style="color:#00d4aa;font-weight:bold">' + esc(c.ssid || c.bssid) + '</span>' +
                (c.bssid ? ' <span style="color:#666;font-size:10px">[' + esc(c.bssid) + ']</span>' : '') +
                '<br><span style="color:#f0c040;font-family:monospace;font-size:12px">' + esc(c.password) + '</span></div>';
        }).join('');
    });
}

// --- Action functions ---

function toggleAttack(name, val) {
    var data = {};
    data[name] = val;
    api('POST', '/api/attacks', data).then(function() {
        toast('Attack ' + name + (val ? ' ON' : ' OFF'));
    });
}
function setRate(r) {
    api('POST', '/api/rate', {rate: r}).then(function() {
        [1,2,3].forEach(function(n) {
            document.getElementById('rate-'+n).classList.toggle('active', n === r);
        });
        toast('Rate set to ' + r);
    });
}
function switchMode(mode) {
    toast('Switching to ' + mode + '...');
    api('POST', '/api/mode', {mode: mode}).then(function(r) {
        if (r && r.ok) toast(r.message);
    });
}
function restartAO() {
    api('POST', '/api/restart', {}).then(function(r) {
        toast(r && r.message ? r.message : 'Restart queued');
    });
}
function doShutdown() {
    api('POST', '/api/shutdown', {}).then(function(r) {
        toast(r && r.message ? r.message : 'Shutdown queued');
    });
}
function restartPwn() {
    api('POST', '/api/restart', {}).then(function(r) {
        toast('Pwnagotchi restart queued');
    });
}

function refreshPlugins() {
    api('GET', '/api/plugins').then(function(plugins) {
        if (!plugins) return;
        var html = '';
        plugins.forEach(function(p) {
            var tagColor = p.tag === 'default' ? '#00d4aa' : '#f0c040';
            html += '<div class="toggle-row">' +
                '<div class="toggle-info">' +
                '<div class="toggle-label">' + esc(p.name) +
                ' <span style="color:' + tagColor + ';font-size:10px;padding:1px 6px;border:1px solid ' + tagColor + ';border-radius:8px;margin-left:6px">' + esc(p.tag) + '</span>' +
                ' <span style="color:#666;font-size:10px;margin-left:4px">v' + esc(p.version) + '</span></div>' +
                '<div class="toggle-desc" style="margin-top:4px">' +
                'x: <input type="number" min="0" max="249" value="' + p.x + '" style="width:48px;background:#0a1628;color:#e0e0e0;border:1px solid #0f3460;border-radius:4px;padding:2px 4px;font-size:11px" onchange="updatePlugin(\'' + esc(p.name) + '\',this.parentNode)">' +
                ' y: <input type="number" min="0" max="121" value="' + p.y + '" style="width:48px;background:#0a1628;color:#e0e0e0;border:1px solid #0f3460;border-radius:4px;padding:2px 4px;font-size:11px" onchange="updatePlugin(\'' + esc(p.name) + '\',this.parentNode)">' +
                '</div></div>' +
                '<label class="switch"><input type="checkbox" ' + (p.enabled ? 'checked' : '') + ' onchange="togglePlugin(\'' + esc(p.name) + '\',this.checked)"><span class="slider"></span></label>' +
                '</div>';
        });
        document.getElementById('plugins-list').innerHTML = html || '<div style="color:#555;font-size:12px">No plugins loaded</div>';
    });
}

function togglePlugin(name, enabled) {
    api('POST', '/api/plugins', [{name: name, enabled: enabled}])
        .then(function(r) { toast('Plugin ' + name + (enabled ? ' ON' : ' OFF')); });
}

function updatePlugin(name, container) {
    var inputs = container.querySelectorAll('input[type=number]');
    var x = parseInt(inputs[0].value) || 0;
    var y = parseInt(inputs[1].value) || 0;
    api('POST', '/api/plugins', [{name: name, x: x, y: y}])
        .then(function(r) { toast(name + ' position: ' + x + ',' + y); });
}

// --- Initial load & auto-refresh ---
refreshStatus();
setTimeout(refreshBattery, 500);
setTimeout(refreshBluetooth, 1000);
setTimeout(refreshWifi, 1500);
setTimeout(refreshAttacks, 2000);
setTimeout(refreshCaptures, 2500);
setTimeout(refreshRecovery, 3000);
setTimeout(refreshPersonality, 3500);
setTimeout(refreshSystem, 4000);
setTimeout(refreshCracked, 4500);
setTimeout(refreshPlugins, 5000);

setInterval(refreshStatus, 5000);
setInterval(refreshBattery, 15000);
setInterval(refreshBluetooth, 15000);
setInterval(refreshWifi, 5000);
setInterval(refreshAttacks, 10000);
setInterval(refreshCaptures, 30000);
setInterval(refreshRecovery, 15000);
setInterval(refreshPersonality, 10000);
setInterval(refreshSystem, 15000);
setInterval(refreshCracked, 60000);
setInterval(refreshPlugins, 15000);
setInterval(function(){ document.getElementById('eink-img').src='/api/display.png?t='+Date.now(); }, 5000);
</script>
</body>
</html>
"##;
