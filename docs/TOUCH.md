# The touchscreen (AXS5106L): what we know, and theories about what's left

The board's touch controller is an AXS5106L on I2C. This page records what was proven on the
board on 2026-10-19, and the open problem: **every touch registers as a long press**, so taps
don't work.

The code is `mod display` in `meritledger_rs/src/bin/s3.rs`, in `Chip::read` and `touch_chip`.
The gesture logic is `meritledger_rs/src/screen/ui.rs` (`Touch::sample`), which is tested on the
desktop. The reference was mydazy's
[esp_lcd_touch_axs5106l](https://github.com/mydazy/esp_lcd_touch_axs5106l) (Apache-2.0), an
ESP-IDF driver for a JD9853 panel with the same chip.

## Proven on the board

| Fact | How we know |
|---|---|
| I2C address 0x63, SDA 42, SCL 41, 400 kHz | The chip ACKs there. |
| **Register reads need a write, a STOP, then a read.** A combined write-read (repeated start) returns all zeros. | With write-read, the ID read `00 00 00`. With write, STOP, read, it reads `510601` (AXS5106, revision 01), firmware 256. The reference driver does the same. |
| **INT is GPIO 48, reset is GPIO 47.** The factory boot log has it the other way round; CircuitPython has it right. | With INT read on 47, nothing ever happened. On 48, touches arrived. Recorded in `microcontroller/BOARD_SKILL_ESP32_S3_TOUCH_LCD_1_47.md`. |
| **While a finger is held, INT pulses; it doesn't stay low.** | The build that treated "INT high" as "lifted" turned a held finger into a stream of taps, and the cards cycled fast. |
| Register 0x01 holds a 6-byte frame: gesture, count, XH, XL, YH, YL. X is `(XH & 0x0F) << 8 \| XL`, and Y likewise. | From the reference driver; the first frames will confirm it (see below). |
| Touch is **not** what broke Wi-Fi that evening. | A build with touch fully disabled joined once, then failed for more than 4 minutes, the same as every touch build. The board was at the edge of the router's range (-79 to -86 dBm when it joined). |

## The open problem: every touch is a long press

The current logic in `Chip::read`:

1. Idle: read nothing until INT goes low. That's a cheap pin check every ~10 ms.
2. INT low: read frames on every tick until the frame's **count is 0**, which counts as the lift.

`Touch::sample` calls it a tap when the lift comes within 700 ms, and a long press otherwise.
Every touch being a long press means the lift is late or never seen. The theories, most likely
first:

### Theory 1: register 0x01 keeps the last report after the finger lifts
The chip writes a frame when it has something to report and pulses INT. Between reports, the
register may simply hold the last frame. If the lift isn't reported as count 0, or is reported
only once and we read it at the wrong moment, `down` stays true. The hold then grows past 700 ms
however short the touch was.

**Test:** the firmware logs its first 12 non-zero frames as `touch: frame [..]` in
`/api/v1/log.txt` (and on the serial console). Tap once, quickly, and look at the last frame
before the next touch. If the count is still 1, this theory holds.

### Theory 2: the lift is an event flag, not a count of 0
Many touch controllers in this family put an **event** in the top two bits of XH: 0 for press,
1 for lift, 2 for contact (FocalTech's convention, which several Chinese controllers copied). We
mask XH with `0x0F` and never look at bits 6–7. If the chip reports the lift as "count 1, event
1", we treat it as the finger still being down.

**Test:** the same frame log. Look at `XH & 0xC0` on the last frame of a tap. Also check
`data[0]` (gesture): the chip may report its own tap or long-press code there, which we ignore.

### Theory 3: lift should be "no INT pulse for a while"
The reference driver decides a finger has lifted on the first poll (every 30 ms) where INT is
high, and reads only while INT is low. On our board, INT pulses during a hold (proven above), so
that rule would see a lift between pulses, which is exactly the fast cycling we saw. The robust
version is a timeout: **a finger is down while INT pulses keep arriving; it has lifted when
there's been no pulse for longer than the report interval** (perhaps 50–100 ms; measure it).

**Test:** log the time between INT falling edges during a hold. That gives the report rate,
and the timeout should be about two to three times that.

### Theory 4 (less likely): the controller's firmware mode
The reference driver checks the firmware version (register 0x05) and can reflash it. Ours reports
256, a plausible factory version. Some modes report only gestures, not points, but we do get
point frames, so this probably isn't it.

## The fix (once the frames confirm a theory)

The likely end state combines theories 2 and 3:

- **Pressed:** INT goes low and a frame with count ≥ 1 arrives (and an event other than "lift",
  if theory 2 holds).
- **Lifted:** a lift event (theory 2), **or** a frame with count 0, **or** no INT pulse for the
  timeout (theory 3), whichever comes first. The timeout guarantees we never get stuck "down".
- Keep `Touch::sample` as it is (tap under 700 ms, long press at 700 ms). Add a desktop test that
  feeds it a press followed by a stale frame, so the stuck-down case can't come back.

`Chip` keeps `down` itself, so all of this fits in `Chip::read` without touching the menu logic.

## Calibration (after taps work)

Raw coordinates are portrait (172 × 320) and the screen is landscape. `ui::landscape` rotates
them. If taps land mirrored, *Household → Messages → The board's screen* shows "Last touch" (raw
point, then on-screen point), and has a checkbox to flip each axis. No reflash is needed.
