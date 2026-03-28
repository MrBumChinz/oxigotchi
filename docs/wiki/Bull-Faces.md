# Bull Faces Reference

← [Back to Wiki Home](Home)

---

Every mood has its own bull. Here are 26 faces:

| Face | Name | What's Happening |
|---|---|---|
| ![awake](faces/eink/awake.png) | **Awake** | System booting or starting a new epoch |
| ![look_r](faces/eink/look_r.png) | **Scanning** | Sweeping channels, looking for targets |
| ![look_r_happy](faces/eink/look_r_happy.png) | **Scanning (happy)** | Sweeping channels, good capture rate |
| ![intense](faces/eink/intense.png) | **Intense** | Sending PMKID association frames |
| ![cool](faces/eink/cool.png) | **Cool** | Sending deauthentication frames |
| ![happy](faces/eink/happy.png) | **Happy** | Just captured a handshake |
| ![excited](faces/eink/excited.png) | **Excited** | On a capture streak |
| ![smart](faces/eink/smart.png) | **Smart** | Found optimal channel or processing logs |
| ![motivated](faces/eink/motivated.png) | **Motivated** | High capture rate |
| ![sad](faces/eink/sad.png) | **Sad** | Long dry spell, no captures |
| ![bored](faces/eink/bored.png) | **Bored** | Nothing happening for a while |
| ![demotivated](faces/eink/demotivated.png) | **Demotivated** | Low success rate |
| ![angry](faces/eink/angry.png) | **Angry** | Very long inactivity or many failed attacks |
| ![lonely](faces/eink/lonely.png) | **Lonely** | No other pwnagotchis nearby |
| ![grateful](faces/eink/grateful.png) | **Grateful** | Active captures + good peer network |
| ![friend](faces/eink/friend.png) | **Friend** | Met another pwnagotchi |
| ![sleep](faces/eink/sleep.png) | **Sleep** | Idle between epochs |
| ![broken](faces/eink/broken.png) | **Broken** | Crash recovery, forced restart |
| ![upload](faces/eink/upload.png) | **Upload** | Sending captures to wpa-sec/wigle |
| ![wifi_down](faces/eink/wifi_down.png) | **WiFi Down** | Monitor interface lost |
| ![fw_crash](faces/eink/fw_crash.png) | **FW Crash** | WiFi firmware crashed, recovering |
| ![ao_crashed](faces/eink/ao_crashed.png) | **AO Crashed** | AngryOxide process died, restarting |
| ![battery_low](faces/eink/battery_low.png) | **Battery Low** | Battery below 20% |
| ![battery_critical](faces/eink/battery_critical.png) | **Battery Critical** | Battery below 15%, shutdown soon |
| ![debug](faces/eink/debug.png) | **Debug** | Debug mode active |
| ![shutdown](faces/eink/shutdown.png) | **Shutdown** | Clean power off |

## Face Selection Logic

The personality engine picks faces based on a combination of factors:

1. **Mood score** — A running score adjusted each epoch based on results:
   - Handshakes captured: big mood boost (+20)
   - Associations seen: moderate boost (+5)
   - Deauths sent: small boost (+2)
   - Blind epoch (no activity): mood decay (-3)
   - Consecutive blind epochs: increasing penalty

2. **XP events** — Level-ups trigger the Excited face regardless of mood

3. **System state overrides** — These always take priority over mood:
   - WiFi firmware crash → FW Crash face
   - AO process crash → AO Crashed face
   - Battery < 20% → Battery Low face
   - Battery < 15% → Battery Critical face
   - WiFi interface lost → WiFi Down face
   - Shutdown command → Shutdown face
   - Uploading captures → Upload face
   - Debug mode → Debug face

4. **Mood-to-face mapping** — When no override is active, the mood score maps to a face:
   - Very high mood (streak) → Excited
   - High mood → Happy, Motivated
   - Neutral → Scanning, Awake, Cool
   - Low mood → Sad, Bored
   - Very low mood → Demotivated, Angry, Lonely

## RF-Driven Mood

The RF classification pipeline feeds real-time spectrum data into the personality engine, adding a new dimension to face selection:

| RF Condition | Mood Effect | Typical Face |
|-------------|-------------|-------------|
| **Busy spectrum** (many beacons, high frame rate) | Mood boost — lots of targets to hunt | Excited, Motivated |
| **Deauth storm** (high deauth_rate) | Mood spike — aggressive environment | Angry, Intense |
| **Silence** (near-zero frame rate) | Mood drain — nothing on the air | Lonely, Bored |
| **Rich BSSIDs** (many unique APs) | Curiosity boost — diverse environment | Smart, Scanning (happy) |
| **Dense data traffic** | Moderate boost — active network | Cool, Motivated |

The RF mood deltas are additive with the existing epoch-based mood system. A bull that captured a handshake in a busy RF environment gets a double mood boost — one from the capture, one from the spectrum richness.
