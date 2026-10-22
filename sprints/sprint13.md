# Sprint 13 — The board's screen and touch menu
**State:** built 2026-10-19; board checks pending (the board was unplugged)
**Depends on:** 1 (board), and for content 3, 7, 10   **Parallel with:** — (the driver half can start right after 1)

## Goal
Make the 172×320 touchscreen a quiet household noticeboard. It shows today's lunar day and the next
observance, notifications, and a Dharma message. Its touch menu offers **call to service** and
**record practice → for whom → what**.

## Read first
- `../microcontroller/BOARD_SKILL_ESP32_S3_TOUCH_LCD_1_47.md` § Pin reference (note the touch INT and RST
  pin conflict)
- `../microcontroller/gamematcher/spec/07-display.md` and `api/src/screen/` (cards, strip drawing,
  fonts, the frame.bmp endpoint)
- Waveshare's factory demo or the CircuitPython port for the JD9853 init sequence and the AXS5106L touch protocol

## Already decided
- The touch menu needs no sign-in (household trust). "Record for whom" lists people from
  `/api/v1/people`, and entries get `recorded_by: board`.
- Quotes and messages come from a bundled set: public-domain sources plus owner-reviewed LLM
  drafts, with provenance.
- `GET /api/v1/display/frame.bmp` shows the current screen for development (copy gamematcher).

## Questions for the owner (answered 2026-10-19)
- **Cards:** keep two in rotation at least: Today (time, lunar date, observance, who has practised)
  and an hourly verse. A fresh call to practice and an event within 7 days join when there is one.
  The owner: "I prefer keeping 2 cards in rotation."
- **No pixel shift** against burn-in (the owner: "I think that would be annoying"). Instead, the
  cards rotate every 10 s and the screen dims to 5% from 22:00 to 06:00.
- **Record menu kinds:** meditation, chanting, prostrations, reading, kept the precepts.

The original questions:

1. Card list and order. The proposal:
   - Today (lunar day and observance)
   - Next observance or feast
   - Household today (who has practised)
   - A notification, if any
   - A message of the day
2. Which kinds belong in the board's "record practice" menu (a short list, about 6)?
3. Screen schedule: dim at night, or off between certain hours?

## Deliverables
- [x] `components/display` (C), as in gamematcher:
  - JD9853 init, a 34-px column offset, and SPI DMA.
  - Backlight PWM on GPIO 46.
  - Strip flush, or a full framebuffer in PSRAM if that's simpler.
- [ ] Touch: AXS5106L over I2C (SCL 41, SDA 42). Resolve the INT and RST pin question on the hardware and update
  the board skill file.
- [x] `screen/`: card deck built from app state, a renderer shared with `frame.bmp`, and a font atlas
  (Atkinson Hyperlegible, as gamematcher uses).
- [x] Touch UI: tap = next card, and long-press or a button opens the menu. Menu flows:
  - Call to service → confirm → `POST /api/v1/call`.
  - Record practice → person → kind → done (with a 5 s undo).
- [x] Touch targets of at least 48 px tall on a 172-px-wide screen. Two-level menus at most.
- [x] `tools/display_mockups.py` to render every card to PNG for review (copy gamematcher's).

## Out of scope
Audio on the board, the 2-inch board, the battery gauge.

## What was built
- **Screen:** the JD9853 driver (`components/display`, init from the CircuitPython port, MIT).
  - Strips of 8 rows (5 KB DMA), drawn by `screen/` with the Atkinson Hyperlegible atlases.
  - Cards: Today, Call, Event and Verse. Menu screens: Buttons and Message.
  - `GET /api/v1/display/frame.bmp?page=` is the same renderer.
- **Touch:** AXS5106L over I2C at 0x63 (protocol from Waveshare's example via the MIT `axs5106l`
  crate). It's polled every 30 ms on the main task.
  - Reset is pin 48, by the factory boot log; INT 47 isn't used.
  - If the chip doesn't answer, the screen works untouched, and the log suggests pin 47.
- **`screen/ui.rs`:** the menu logic, pure and tested on the desktop.
  - A tap moves to the next card. A 0.7 s hold opens the menu: Record practice → who (5 per page,
    More) → what → "Recorded" with a 5 s Undo; or Call to practice → confirm.
  - The menu closes after 30 s idle.
  - Buttons are at least 48 px tall in a 2-row grid, and each menu uses one font size.
- **App hooks:**
  - `record_from_board` records as the board.
  - `undo_from_board` takes back only the board's own entries, from the last 60 s.
  - `call_from_board` sends "Someone at the board calls everyone to practice".
- **Redraws** happen only on a gesture, a menu timeout, a card turn or a new minute. The average
  redraw time is logged every 100 draws.
- **Calibration without reflashing:** `touch_flip_x`/`touch_flip_y` (StoredState 41, 42) and the
  last touch (raw → on screen) on the admin Messages page.
- **`tools/display_mockups.py`** renders every card and menu through the firmware's own renderer
  into `spec/display/` (`sheet.png` has them all).
- **`unsafe`:** all the screen FFI is in one `Panel` type, with a once-per-boot guard and a null
  check on the driver's buffer, which was unchecked before.

## Exit criteria
- [ ] Mockups reviewed by the owner: `spec/display/sheet.png`.
- [ ] On the board, a touch-menu record shows on Today within 2 s. The board was unplugged; the
  same path is tested on the desktop (`tests/notify.rs`, `the_touch_menu_records_undoes_and_calls`).
- [ ] Redraw time and heap, from the log line `screen: N redraws, M ms each` and `/api/v1/sys`.
- [ ] A 24 h run with the screen on.

## Handoff
- **On the board, 2026-10-19:** the touch pins are INT 48 and reset 47, and reads use
  write-STOP-read. Touches register, but always as long presses. The theories and tests are in
  [docs/TOUCH.md](../docs/TOUCH.md).
- **First board run** (needs the owner and `make update PORT=COM5`):
  1. Look for `touch: AXS5106L answers`. If it's missing, try reset pin 47 and update
     `BOARD_SKILL_ESP32_S3_TOUCH_LCD_1_47.md` with whichever works.
  2. Hold the screen: the menu should open. If taps land mirrored, use the flips on the admin
     page.
  3. Record something, check Today on a phone, and undo it.
- The touch menu needs no sign-in (household trust, as decided). Board entries have
  `recorded_by` 0.
