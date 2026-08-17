import fs from 'node:fs';

const args = Object.fromEntries(process.argv.slice(2).map((argument) => {
  const separator = argument.indexOf('=');
  return separator === -1 ? [argument.replace(/^--/, ''), true] : [argument.slice(2, separator), argument.slice(separator + 1)];
}));
const locale = String(args.locale ?? '');
const language = String(args.language ?? '');
const prefix = typeof args.prefix === 'string' ? args.prefix : '';
const model = String(args.model ?? 'translategemma:4b');
if (!locale || !language) throw new Error('Use --locale=<code> and --language=<name>.');

const english = JSON.parse(fs.readFileSync('src/locales/en.json', 'utf8'));
const targetPath = `src/locales/${locale}.json`;
const target = !args.reset && fs.existsSync(targetPath) ? JSON.parse(fs.readFileSync(targetPath, 'utf8')) : {};
const progressPath = `/tmp/pasted-${locale}-translation-progress.json`;
const completed = new Set(!args.reset && fs.existsSync(progressPath) ? JSON.parse(fs.readFileSync(progressPath, 'utf8')) : []);

const write = () => {
  const ordered = Object.fromEntries(Object.keys(english).map((key) => [key, target[key] ?? english[key]]));
  fs.writeFileSync(targetPath, `${JSON.stringify(ordered, null, 2)}\n`);
  fs.writeFileSync(progressPath, JSON.stringify([...completed]));
};

function mask(message, key) {
  const placeholders = [];
  let text = message;
  const protect = (pattern) => {
    text = text.replace(pattern, (placeholder) => {
    const token = `ZXQPH${placeholders.length}QXZ`;
    placeholders.push([token, placeholder]);
    return token;
    });
  };
  // Only syntax that must survive byte-for-byte is masked. TranslateGemma
  // preserves product and technical names more reliably when it can see them;
  // masking a long series of those terms can cause it to renumber opaque tokens.
  protect(/\{[A-Za-z][A-Za-z0-9_]*\}|https?:\/\/\S+|--[a-z][a-z0-9-]*/g);
  if (!/^component\.activityLogView\.(?:hudPasted|queuePasted)$/.test(key)) protect(/\bPasted\b/g);
  return { text, placeholders };
}

const shouldTranslate = (message) => /[A-Za-z]/.test(message)
  && !/^(?:https?:\/\/|\/[^ ]|[A-Z0-9_]+\s*=)/.test(message);

async function requestTranslation(text) {
  const prompt = [
    `You are a professional English (en) to ${language} (${locale}) translator. Your goal is to accurately convey the meaning and nuances of the original English text while adhering to ${language} grammar, vocabulary, and cultural sensitivities.`,
    `Produce only the ${language} translation, without any additional explanations or commentary. Please translate the following English text into ${language}:`,
    '',
    '',
    text,
  ].join('\n');
  const response = await fetch('http://127.0.0.1:11434/api/generate', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ model, prompt, stream: false, keep_alive: '30m', options: { temperature: 0 } }),
  });
  if (!response.ok) throw new Error(`Ollama returned ${response.status}: ${await response.text()}`);
  return String((await response.json()).response ?? '').trim().replace(/^['"]|['"]$/g, '');
}

function restore(key, translated, placeholders) {
  for (const [token, placeholder] of placeholders) {
    if (!translated.includes(token)) throw new Error(`${key} lost placeholder ${placeholder}: ${translated}`);
    translated = translated.replaceAll(token, placeholder);
  }
  return translated;
}

async function translateMessage(key, message) {
  if (!shouldTranslate(message)) return message;
  const { text, placeholders } = mask(message, key);
  return restore(key, await requestTranslation(text), placeholders) || message;
}

async function translateBatch(items) {
  const translatable = items.filter(({ message }) => shouldTranslate(message));
  const results = new Map(items.filter(({ message }) => !shouldTranslate(message)).map(({ id, message }) => [id, message]));
  if (translatable.length === 0) return results;
  const masked = translatable.map(({ id, key, message }) => ({ id, key, message, ...mask(message, key) }));
  const combined = masked.map(({ text }, index) => `[${index}] ${text}`).join('\n');
  try {
    const translated = await requestTranslation(combined);
    const markers = [...translated.matchAll(/^\[(\d+)\]\s*/gm)];
    if (markers.length !== masked.length) throw new Error(`batch returned ${markers.length} of ${masked.length} numbered items`);
    const parts = new Array(masked.length);
    for (let index = 0; index < markers.length; index += 1) {
      const itemIndex = Number(markers[index][1]);
      const start = (markers[index].index ?? 0) + markers[index][0].length;
      const end = markers[index + 1]?.index ?? translated.length;
      parts[itemIndex] = translated.slice(start, end).trim();
    }
    if (parts.some((part) => !part)) throw new Error('batch response contained an empty or missing item');
    masked.forEach((item, index) => results.set(item.id, restore(item.key, parts[index], item.placeholders) || item.message));
  } catch (error) {
    process.stderr.write(`\nBatch validation failed (${error.message}); retrying individually.\n`);
    for (const item of masked) results.set(item.id, await translateMessage(item.key, item.message));
  }
  return results;
}

let processed = 0;
const batchSize = Number(args['batch-size'] ?? 25);
const pending = Object.entries(english).filter(([key, message]) => {
  if (prefix && !key.startsWith(prefix)) return false;
  if (completed.has(key)) return false;
  if (args['only-identical'] && JSON.stringify(target[key]) !== JSON.stringify(message)) {
    completed.add(key);
    return false;
  }
  return true;
});
for (let offset = 0; offset < pending.length; offset += batchSize) {
  const entries = pending.slice(offset, offset + batchSize);
  const items = entries.flatMap(([key, message]) => typeof message === 'string'
    ? [{ id: key, key, message }]
    : Object.entries(message).map(([category, variant]) => ({ id: `${key}.${category}`, key: `${key}.${category}`, message: variant })));
  const translations = await translateBatch(items);
  for (const [key, message] of entries) {
    target[key] = typeof message === 'string'
      ? translations.get(key)
      : Object.fromEntries(Object.keys(message).map((category) => [category, translations.get(`${key}.${category}`)]));
    completed.add(key);
    processed += 1;
  }
  write();
  process.stdout.write(`\rTranslated ${processed} messages for ${locale}…`);
}
write();
console.log(`\nTranslated ${processed} messages for ${locale}.`);
