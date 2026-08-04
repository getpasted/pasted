/**
 * Lightweight CSS architecture guard for Pasted.
 *
 * This intentionally measures migration debt as well as correctness. Lower the
 * budgets whenever a cleanup wave lands so specificity cannot silently regrow.
 */
import fs from 'node:fs';

const IMPORTANT_BUDGET = 11;
const COMPATIBILITY_SELECTOR_BUDGET = 1;
const UTILITY_COUPLED_SELECTOR_BUDGET = 0;
const HARD_CODED_SURFACE_BUDGET = 0;

const readFilesRecursively = (directory, extension) => fs.readdirSync(directory, { withFileTypes: true })
  .flatMap((entry) => {
    const path = `${directory}/${entry.name}`;
    if (entry.isDirectory()) return readFilesRecursively(path, extension);
    return entry.name.endsWith(extension) ? [fs.readFileSync(path, 'utf8')] : [];
  });

const css = readFilesRecursively('src', '.css').join('\n');
const componentSource = readFilesRecursively('src/components', '.tsx').join('\n');
const appCss = fs.readFileSync('src/App.css', 'utf8');
const styleModuleFiles = fs.readdirSync('src/styles')
  .filter((file) => file.endsWith('.css'))
  .sort();
const importedStyleModules = [...appCss.matchAll(/@import\s+"\.\/styles\/([^";]+\.css)";/g)]
  .map((match) => match[1]);
const missingStyleImports = styleModuleFiles.filter((file) => !importedStyleModules.includes(file));
const staleStyleImports = importedStyleModules.filter((file) => !styleModuleFiles.includes(file));
const duplicateStyleImports = importedStyleModules.filter((file, index) => importedStyleModules.indexOf(file) !== index);

const definitions = new Set(
  [...css.matchAll(/^\s*(--[a-zA-Z0-9-]+)\s*:/gm)].map((match) => match[1]),
);
const usages = new Set(
  [...css.matchAll(/var\(\s*(--[a-zA-Z0-9-]+)/g)].map((match) => match[1]),
);
const undefinedTokens = [...usages].filter((token) => !definitions.has(token)).sort();
const importantCount = (css.match(/!important/g) || []).length;
const compatibilitySelectorCount = (css.match(/html:is\(\.cool, \.warm\)/g) || []).length;
const utilityCoupledSelectorCount = (css.match(/\[class~="|\.(?:bg|text|border)-\\\[/g) || []).length;
const hardCodedSurfaceCount = (componentSource.match(/(?:bg|border)-(?:gray|slate|zinc|neutral)-(?:700|800|900)(?:\/[0-9]+)?|(?:bg|border)-\[#[0-9a-fA-F]{3,8}(?:\]\/[0-9]+|\])/g) || []).length;

console.log('\nCSS architecture audit');
console.log('----------------------');
console.log(`!important declarations: ${importantCount}/${IMPORTANT_BUDGET}`);
console.log(`Cool/Warm compatibility selectors: ${compatibilitySelectorCount}/${COMPATIBILITY_SELECTOR_BUDGET}`);
console.log(`Utility-coupled selectors: ${utilityCoupledSelectorCount}/${UTILITY_COUPLED_SELECTOR_BUDGET}`);
console.log(`Hard-coded JSX surfaces: ${hardCodedSurfaceCount}/${HARD_CODED_SURFACE_BUDGET}`);
console.log(`CSS modules imported: ${importedStyleModules.length}/${styleModuleFiles.length}`);

let failed = false;

if (undefinedTokens.length > 0) {
  failed = true;
  console.error(`Undefined custom properties: ${undefinedTokens.join(', ')}`);
}

if (missingStyleImports.length > 0 || staleStyleImports.length > 0 || duplicateStyleImports.length > 0) {
  failed = true;
  if (missingStyleImports.length > 0) console.error(`Unimported CSS modules: ${missingStyleImports.join(', ')}`);
  if (staleStyleImports.length > 0) console.error(`Stale CSS imports: ${staleStyleImports.join(', ')}`);
  if (duplicateStyleImports.length > 0) console.error(`Duplicate CSS imports: ${duplicateStyleImports.join(', ')}`);
}

if (importantCount > IMPORTANT_BUDGET) {
  failed = true;
  console.error(`!important budget exceeded by ${importantCount - IMPORTANT_BUDGET}.`);
}

if (compatibilitySelectorCount > COMPATIBILITY_SELECTOR_BUDGET) {
  failed = true;
  console.error(`Compatibility-selector budget exceeded by ${compatibilitySelectorCount - COMPATIBILITY_SELECTOR_BUDGET}.`);
}

if (utilityCoupledSelectorCount > UTILITY_COUPLED_SELECTOR_BUDGET) {
  failed = true;
  console.error(`Utility-coupled selector budget exceeded by ${utilityCoupledSelectorCount - UTILITY_COUPLED_SELECTOR_BUDGET}.`);
}

if (hardCodedSurfaceCount > HARD_CODED_SURFACE_BUDGET) {
  failed = true;
  console.error(`Hard-coded JSX surface budget exceeded by ${hardCodedSurfaceCount - HARD_CODED_SURFACE_BUDGET}.`);
}

if (failed) process.exit(1);
console.log('CSS architecture audit passed.');
