import type { AppSettings } from '../types';

export function resolveAppTheme(theme: AppSettings['themeMode']) {
  return theme === 'system'
    ? (window.matchMedia('(prefers-color-scheme: light)').matches ? 'cool' : 'dark')
    : theme;
}

export function applyAppTheme(theme: AppSettings['themeMode']) {
  const resolvedTheme = resolveAppTheme(theme || 'system');
  const root = document.documentElement;
  root.dataset.theme = resolvedTheme;
  root.classList.toggle('cool', resolvedTheme === 'cool');
  root.classList.toggle('dark', resolvedTheme === 'dark');
  root.classList.toggle('warm', resolvedTheme === 'warm');
  root.classList.toggle('theme-2894', resolvedTheme === '2894');
  root.classList.toggle('theme-sauced', resolvedTheme === 'sauced');
  root.classList.toggle('vampire', resolvedTheme === 'vampire');
  root.classList.toggle('flux', resolvedTheme === 'flux');
  root.classList.toggle('theme-808', resolvedTheme === '808');
  return resolvedTheme;
}
