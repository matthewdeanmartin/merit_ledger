import { bootstrapApplication } from '@angular/platform-browser';
import { ErrorHandler, inject, Injectable, provideBrowserGlobalErrorListeners, provideZonelessChangeDetection } from '@angular/core';
import { CanActivateFn, provideRouter, Router, Routes, withComponentInputBinding } from '@angular/router';
import { Api } from './app/api';
import { App } from './app/app';
import { UpdateRecovery } from './app/update-recovery';
import { Home } from './app/pages/home';

async function ready(api: Api): Promise<void> {
  if (!api.ready()) await api.refresh();
}

/** Signed-in pages: set the board up first, then sign in. */
const signedIn: CanActivateFn = async () => {
  const api = inject(Api);
  const router = inject(Router);
  await ready(api);
  if (api.status()?.setup_needed) return router.parseUrl('/setup');
  return api.me() ? true : router.parseUrl('/signin');
};

/** Sign-in pages: pointless once signed in. */
const signedOut: CanActivateFn = async () => {
  const api = inject(Api);
  const router = inject(Router);
  await ready(api);
  if (api.status()?.setup_needed) return router.parseUrl('/setup');
  return api.me() ? router.parseUrl('/') : true;
};

const setupOnly: CanActivateFn = async () => {
  const api = inject(Api);
  const router = inject(Router);
  await ready(api);
  return api.status()?.setup_needed ? true : router.parseUrl('/');
};

/** Public pages: anyone on the Wi-Fi, once the board is set up. */
const boardReady: CanActivateFn = async () => {
  const api = inject(Api);
  const router = inject(Router);
  await ready(api);
  return api.status()?.setup_needed ? router.parseUrl('/setup') : true;
};

const adminOnly: CanActivateFn = async () => {
  const api = inject(Api);
  const router = inject(Router);
  await ready(api);
  return api.me()?.role === 'admin' ? true : router.parseUrl('/');
};

/**
 * A board update renames the hashed chunks; an open tab then fails to load
 * one. Turn that into a single reload instead of a broken page.
 */
@Injectable()
class GlobalErrorHandler implements ErrorHandler {
  private readonly recovery = inject(UpdateRecovery);

  handleError(error: unknown): void {
    if (!this.recovery.recover(error)) console.error(error);
  }
}

// Path routing: the board serves index.html for every extension-less path.
const routes: Routes = [
  { path: '', component: Home, canActivate: [signedIn], title: 'Merit Ledger' },
  // Sign-in pages load on demand: someone signed in never needs them.
  {
    path: 'setup',
    canActivate: [setupOnly],
    loadComponent: () => import('./app/pages/auth').then((m) => m.Setup),
    title: 'Set up · Merit Ledger',
  },
  {
    path: 'signin',
    canActivate: [signedOut],
    loadComponent: () => import('./app/pages/auth').then((m) => m.SignIn),
    title: 'Sign in · Merit Ledger',
  },
  {
    path: 'signin/finishing',
    loadComponent: () => import('./app/pages/finishing').then((m) => m.Finishing),
    title: 'Signing in · Merit Ledger',
  },
  {
    path: 'signup',
    canActivate: [signedOut],
    loadComponent: () => import('./app/pages/auth').then((m) => m.SignUp),
    title: 'Guest account · Merit Ledger',
  },
  {
    path: 'house',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/house').then((m) => m.House),
    title: 'The house · Merit Ledger',
  },
  {
    path: 'p/:id',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/person').then((m) => m.Person),
    title: 'Profile · Merit Ledger',
  },
  {
    path: 'books',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/books').then((m) => m.Books),
    title: 'Books · Merit Ledger',
  },
  {
    path: 'events',
    canActivate: [signedIn],
    loadComponent: () => import('./app/pages/events').then((m) => m.Events),
    title: 'Events · Merit Ledger',
  },
  {
    path: 'events/:id',
    canActivate: [signedIn],
    loadComponent: () => import('./app/pages/events').then((m) => m.Events),
    title: 'Event · Merit Ledger',
  },
  {
    path: 'e/:id',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/guest-rsvp').then((m) => m.GuestRsvp),
    title: 'Invitation · Merit Ledger',
  },
  {
    path: 'notifications',
    canActivate: [signedIn],
    loadComponent: () => import('./app/pages/notifications').then((m) => m.Notifications),
    title: 'Notifications · Merit Ledger',
  },
  {
    path: 'build',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/build').then((m) => m.Build),
    title: 'Practices · Merit Ledger',
  },
  {
    path: 'build/:id',
    canActivate: [signedIn],
    loadComponent: () => import('./app/pages/build').then((m) => m.Build),
    title: 'Build a practice · Merit Ledger',
  },
  {
    path: 'practice/:id/:page',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/follow').then((m) => m.Follow),
    title: 'Practice · Merit Ledger',
  },
  {
    path: 'texts',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/texts-page').then((m) => m.TextsPage),
    title: 'Texts · Merit Ledger',
  },
  {
    path: 'texts/:id',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/texts-page').then((m) => m.TextsPage),
    title: 'Texts · Merit Ledger',
  },
  {
    path: 'v/:id',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/vow-page').then((m) => m.VowPage),
    title: 'Vow · Merit Ledger',
  },
  {
    path: 'precepts',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/precepts-page').then((m) => m.PreceptsPage),
    title: 'Precepts · Merit Ledger',
  },
  {
    path: 'precepts/:set',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/precepts-page').then((m) => m.PreceptsPage),
    title: 'Precepts · Merit Ledger',
  },
  {
    path: 'calendar',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/calendar-page').then((m) => m.CalendarPage),
    title: 'Calendar · Merit Ledger',
  },
  {
    path: 'timer',
    canActivate: [boardReady],
    loadComponent: () => import('./app/pages/timer-page').then((m) => m.TimerPage),
    title: 'Timer · Merit Ledger',
  },
  {
    path: 'practices',
    canActivate: [signedIn],
    loadComponent: () => import('./app/pages/practices').then((m) => m.Practices),
    title: 'Practices · Merit Ledger',
  },
  {
    path: 'me',
    canActivate: [signedIn],
    loadComponent: () => import('./app/pages/profile').then((m) => m.Profile),
    title: 'You · Merit Ledger',
  },
  {
    path: 'admin/messages',
    canActivate: [signedIn, adminOnly],
    loadComponent: () => import('./app/pages/messages-admin').then((m) => m.MessagesAdmin),
    title: 'Messages · Merit Ledger',
  },
  {
    path: 'admin',
    canActivate: [signedIn, adminOnly],
    loadComponent: () => import('./app/pages/admin').then((m) => m.Admin),
    title: 'Household · Merit Ledger',
  },
  {
    path: 'guide',
    loadComponent: () => import('./app/pages/guide').then((m) => m.Guide),
    title: 'Guide · Merit Ledger',
  },
  { path: '**', redirectTo: '' },
];

bootstrapApplication(App, {
  providers: [
    provideBrowserGlobalErrorListeners(),
    provideZonelessChangeDetection(),
    provideRouter(routes, withComponentInputBinding()),
    { provide: ErrorHandler, useClass: GlobalErrorHandler },
  ],
}).catch((err) => console.error(err));
