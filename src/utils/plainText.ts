const NON_VISIBLE_HTML = 'script, style, template, noscript';

export function htmlToPlainText(value: string): string {
  const parsed = new DOMParser().parseFromString(value, 'text/html');
  parsed.querySelectorAll(NON_VISIBLE_HTML).forEach((element) => element.remove());
  return parsed.body.textContent ?? '';
}
