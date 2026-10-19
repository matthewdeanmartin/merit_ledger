# Sprint 13 — The board's screen and touch menu
**State:** not started
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

## Questions for the owner
1. Card list and order. The proposal:
   - Today (lunar day and observance)
   - Next observance or feast
   - Household today (who has practised)
   - A notification, if any
   - A message of the day
2. Which kinds belong in the board's "record practice" menu (a short list, about 6)?
3. Screen schedule: dim at night, or off between certain hours?

## Deliverables
- [ ] `components/display` (C), as in gamematcher:
  - JD9853 init, a 34-px column offset, and SPI DMA.
  - Backlight PWM on GPIO 46.
  - Strip flush, or a full framebuffer in PSRAM if that's simpler.
- [ ] Touch: AXS5106L over I2C (SCL 41, SDA 42). Resolve the INT and RST pin question on the hardware and update
  the board skill file.
- [ ] `screen/`: card deck built from app state, a renderer shared with `frame.bmp`, and a font atlas
  (Atkinson Hyperlegible, as gamematcher uses).
- [ ] Touch UI: tap = next card, and long-press or a button opens the menu. Menu flows:
  - Call to service → confirm → `POST /api/v1/call`.
  - Record practice → person → kind → done (with a 5 s undo).
- [ ] Touch targets of at least 48 px tall on a 172-px-wide screen. Two-level menus at most.
- [ ] `tools/display_mockups.py` to render every card to PNG for review (copy gamematcher's).

## Exit criteria
- [ ] Mockups of every card and menu screen are reviewed by the owner.
- [ ] On the board: recording a practice from the touch menu shows on the person's Today page within 2 s.
- [ ] The screen redraws only when something visible changes. Measure the redraw time and the heap.
- [ ] A long run of 24 h with the screen on: no reboot, and heap stable.

## Out of scope
Audio on the board, the 2-inch board, the battery gauge.

## Handoff
_(fill in)_
