# The phone timer (Sprint 6): what to try, and what happened

The timer runs in the phone's browser. Only real phones can show whether the screen stays on,
the bells ring on time, and a locked phone still logs the sit. This page is the checklist, with
blanks for the results.

**Before starting:** the phone should open `https://meritledger.local/` without a certificate warning.
If it shows one, install the household certificate from `http://meritledger.local/trust` first.
Over plain HTTP the timer still works, but it can't keep the screen on and says so.

## 1. A 20-minute sit, screen on

Today → 🔔 Timer → "20-minute sit" → Start. Leave the phone face up and don't touch it.

| | iPhone (Safari) | Android (Chrome) |
|---|---|---|
| Date, OS and browser versions | | |
| Three opening strikes, about 5 s apart | | |
| Screen stayed on the whole time | | |
| Closing bell at 20:00 (within 1 s?) | | |
| "Logged." and Meditation · 20 min on Today | | |

## 2. Locked mid-sit

Start a 10-minute sit. After about 2 minutes, press the power button. Unlock after about 12 minutes.

| | iPhone (Safari) | Android (Chrome) |
|---|---|---|
| Did the closing bell sound while locked? | | |
| On unlock: did it show done and log 10 min? | | |
| Anything odd (late bell, double bell, no log) | | |

What the code expects:
- The session's time comes from the wall clock, so on unlock it knows the sit ended at 10:00, and it
  logs 10 minutes even if no bell rang.
- Whether a bell sounds while locked is up to the browser: iOS usually suspends web audio when
  locked, and Android often keeps it.

## 3. Off the Wi-Fi

Start a 1-minute custom timer, turn Wi-Fi off before it ends, let it finish, then turn Wi-Fi back on
and open Today.

| | Result |
|---|---|
| "Saved on this phone…" at the end | |
| The entry appears once (not twice) after Wi-Fi returns | |

## 4. Mala (108 daimoku)

Start "108 daimoku (mala)" and tap the circle.

| | Result |
|---|---|
| A short buzz per tap, a longer one at 27, 54 and 81 (Android; iPhones don't vibrate from web pages) | |
| Three bells and "Daimoku · 108×" at 108 | |
