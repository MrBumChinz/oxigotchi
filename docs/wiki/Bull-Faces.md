# Bull Faces Reference

← [Back to Wiki Home](Home)

---

The bull has 26 faces. The personality engine selects one based on mood, system state, and RF environment.

| Face | Name | What's Happening |
|---|---|---|
| ![awake](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/awake.png) | **Awake** | System booting or starting a new loop cycle |
| ![intense](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/intense.png) | **Intense** | Sending PMKID association frames |
| ![cool](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/cool.png) | **Cool** | Sending deauthentication frames |
| ![happy](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/happy.png) | **Happy** | Just captured a handshake |
| ![excited](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/excited.png) | **Excited** | On a capture streak |
| ![smart](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/smart.png) | **Smart** | Found optimal channel or processing logs |
| ![motivated](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/motivated.png) | **Motivated** | High capture rate |
| ![sad](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/sad.png) | **Sad** | Long dry spell, no captures |
| ![bored](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/bored.png) | **Bored** | Nothing happening for a while |
| ![demotivated](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/demotivated.png) | **Demotivated** | Low success rate |
| ![angry](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/angry.png) | **Angry** | Very long inactivity or many failed attacks |
| ![lonely](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/lonely.png) | **Lonely** | No other pwnagotchis nearby |
| ![grateful](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/grateful.png) | **Grateful** | Active captures + good peer network |
| ![friend](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/friend.png) | **Friend** | Met another pwnagotchi |
| ![sleep](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/sleep.png) | **Sleep** | Idle, nothing to do |
| ![broken](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/broken.png) | **Broken** | Crash recovery, forced restart |
| ![upload](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/upload.png) | **Upload** | Sending captures to wpa-sec/wigle |
| ![wifi_down](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/wifi_down.png) | **WiFi Down** | Monitor interface lost |
| ![fw_crash](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/fw_crash.png) | **FW Crash** | WiFi firmware crashed, recovering |
| ![ao_crashed](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/ao_crashed.png) | **AO Crashed** | AngryOxide process died, restarting |
| ![battery_low](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/battery_low.png) | **Battery Low** | Battery ≤ 20% |
| ![battery_critical](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/battery_critical.png) | **Battery Critical** | Battery ≤ 5%, shutdown imminent |
| ![raging](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/raging.png) | **Raging** | Entering BT attack mode or deauth storm detected |
| ![grazing](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/grazing.png) | **Grazing** | Calm idle, low activity in SAFE mode |
| ![debug](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/debug.png) | **Debug** | Debug mode active |
| ![shutdown](https://raw.githubusercontent.com/CoderFX/oxigotchi/master/faces/eink/shutdown.png) | **Shutdown** | Clean power off |

## Face Selection Logic

The personality engine picks faces based on a combination of factors:

1. **Mood score** — A 0-100% score that shifts based on events:
   - Handshake captured: +5% mood boost
   - New AP discovered: +0.5% per AP (capped at 10 APs per cycle)
   - AO or firmware crash: -3% mood penalty
   - Idle time: gradual random-walk decay via 30-second mood ticks

2. **XP events** — Level-ups give a +8% mood boost (but don't force a specific face)

3. **System state overrides** — These always take priority over mood:
   - WiFi firmware crash → FW Crash face
   - AO process crash → AO Crashed face
   - Battery ≤ 20% → Battery Low face
   - Battery ≤ 5% → Battery Critical face
   - WiFi interface lost → WiFi Down face
   - Shutdown command → Shutdown face
   - Uploading captures → Upload face
   - Debug mode → Debug face

4. **Mood-to-face mapping** — When no override is active, the mood score maps to a face:
   - Very high mood (streak) → Excited
   - High mood → Happy, Motivated
   - Neutral → Awake, Cool
   - Low mood → Sad, Bored
   - Very low mood → Demotivated, Angry, Lonely

## RF-Driven Mood

The RF classification pipeline feeds real-time spectrum data into the personality engine, adding a new dimension to face selection:

| RF Condition | Mood Effect | Typical Face |
|-------------|-------------|-------------|
| **Busy spectrum** (many beacons, high frame rate) | Mood boost — lots of targets to hunt | Excited, Motivated |
| **Deauth storm** (high deauth_rate) | Mood spike — aggressive environment | Angry, Intense |
| **Silence** (near-zero frame rate) | Mood drain — nothing on the air | Lonely, Bored |
| **Rich BSSIDs** (many unique APs) | Curiosity boost — diverse environment | Smart |
| **Dense data traffic** | Moderate boost — active network | Cool, Motivated |

The RF mood deltas are additive with the base mood system. A bull that captured a handshake in a busy RF environment gets a double mood boost — one from the capture, one from the spectrum richness.
