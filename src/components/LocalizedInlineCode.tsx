interface LocalizedInlineCodeProps {
  message: string;
  code: string;
}

export function LocalizedInlineCode({ message, code }: LocalizedInlineCodeProps) {
  const codeIndex = message.indexOf(code);
  if (codeIndex < 0) return <>{message}</>;

  let before = message.slice(0, codeIndex);
  let after = message.slice(codeIndex + code.length);
  if (before.endsWith('\u2068') && after.startsWith('\u2069')) {
    before = before.slice(0, -1);
    after = after.slice(1);
  }

  return <>{before}<code dir="ltr" className="theme-code-surface inline-block rounded border px-1 py-px align-baseline font-mono text-[1.05em] font-semibold leading-[1.25]">{code}</code>{after}</>;
}
