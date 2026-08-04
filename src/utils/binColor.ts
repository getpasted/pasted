export function binTextColor(color?: string | null): string | undefined {
  if (!color || color === 'default') return undefined;
  return /^#[0-9a-f]{6}$/i.test(color) ? color : undefined;
}
