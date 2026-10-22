import { Component } from '@angular/core';

/**
 * `/guide`: the one-page household guide (Sprint 14). Open to everyone,
 * signed in or not, and laid out to print on one sheet (the print CSS hides
 * the header, nav and footer).
 */
@Component({
  selector: 'ml-guide',
  styles: `
    h2 { margin-top: 1.25rem; font-size: 1.1rem; }
    ul { padding-left: 1.2rem; }
    li { margin: 0.2rem 0; }
    @media print {
      :host { font-size: 11pt; line-height: 1.35; }
      .noprint { display: none; }
      h1 { font-size: 16pt; margin: 0 0 0.25rem; }
      h2 { font-size: 12pt; margin: 0.6rem 0 0.15rem; }
    }
  `,
  template: `
    <article class="card">
      <h1>Merit Ledger: a guide for the household</h1>
      <p>Our practice log lives on the little screen in the shrine room. Open it on any phone on the home Wi-Fi at
        <strong>meritledger.local</strong>.</p>

      <h2>Every day</h2>
      <ul>
        <li><strong>Log a practice:</strong> on <em>Today</em>, tap a tile (Meditation, Chanting, Reading…). Tap again
          to log another. Made a mistake? Open the entry and change or delete it.</li>
        <li><strong>Sit with the timer:</strong> <em>Timer</em> rings the bells and logs the minutes for you when you
          finish.</li>
        <li><strong>Follow a practice:</strong> <em>Practice</em> has the household's liturgies, page by page, big
          enough to read from the cushion.</li>
        <li><strong>Rejoice:</strong> tap 🙏 on someone's entry in the house feed.</li>
      </ul>

      <h2>The screen</h2>
      <ul>
        <li>It shows the time, the lunar date, the next observance and a verse, turning every ten seconds.
          It dims at night.</li>
        <li><strong>Hold it for a second</strong> to open its menu: <em>Record practice</em> (pick who, then what) or
          <em>Call to practice</em>. <em>Undo</em> appears for a few seconds after recording.</li>
        <li>No sign-in is needed at the screen: it trusts whoever is in the house.</li>
      </ul>

      <h2>Together</h2>
      <ul>
        <li><strong>Call to practice:</strong> rings the bell 🔔 on everyone's phone (and sends a message, if the
          household has set that up).</li>
        <li><strong>Events:</strong> plan a potluck or a retreat day, invite people, and send guests a link to answer.</li>
        <li><strong>Vows and precepts:</strong> take a vow, keep it, and mark how it went.</li>
        <li><strong>Books:</strong> keep your reading list and log reading sessions.</li>
      </ul>

      <h2>Your account</h2>
      <ul>
        <li><strong>Three ways in:</strong> sign in with your <strong>PIN</strong>; or, visiting, start a
          <strong>guest visit</strong> with just your name (it lives on your phone, and it's cleared after 90
          days unused); or just look around: practices, texts, the calendar, the timer and books need no
          account.</li>
        <li>Forgot your PIN? Ask a household admin to give you a new one.</li>
        <li>Choose who sees each entry: just you, the household, or everyone (public, which includes guests and
          visitors).</li>
        <li><em>You</em> holds your profile, reminders and PIN.</li>
      </ul>

      <h2>If something's wrong</h2>
      <ul>
        <li><strong>"The board is not responding":</strong> check you're on the home Wi-Fi, then that the board has power.</li>
        <li><strong>"The clock isn't set yet":</strong> the board just started; give it a minute online.</li>
        <li><strong>"Too many at once":</strong> wait the few seconds it says and try again.</li>
        <li>Admins: backups go to the SD card nightly, and <em>Household</em> can export everything.</li>
      </ul>

      <p class="noprint"><button type="button" (click)="print()">Print this page</button></p>
    </article>
  `,
})
export class Guide {
  print(): void {
    window.print();
  }
}
