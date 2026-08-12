/**
 * Lightweight CSS architecture guard for Pasted.
 *
 * This intentionally measures migration debt as well as correctness. Lower the
 * budgets whenever a cleanup wave lands so specificity cannot silently regrow.
 */
import fs from 'node:fs';

const IMPORTANT_BUDGET = 4;
const COMPATIBILITY_SELECTOR_BUDGET = 1;
const UTILITY_COUPLED_SELECTOR_BUDGET = 0;
const HARD_CODED_SURFACE_BUDGET = 0;
const DEFAULT_PALETTE_UTILITY_BUDGET = 0;
const HARD_CODED_THEME_COLOR_BUDGET = 0;
const LEGACY_FIELD_RADIUS_BUDGET = 0;
const RAW_PILL_RADIUS_BUDGET = 0;
const SEMANTIC_RADIUS_OVERRIDE_BUDGET = 0;

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
const themeCssOutsideFoundation = styleModuleFiles
  .filter((file) => file !== 'foundation.css')
  .map((file) => fs.readFileSync(`src/styles/${file}`, 'utf8'))
  .join('\n');
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
const compatibilitySelectorCount = (css.match(/html:is\(\.cool, \.warm, \.theme-2894, \.theme-sauced\)/g) || []).length;
const utilityCoupledSelectorCount = (css.match(/\[class~="|\.(?:bg|text|border)-\\\[/g) || []).length;
const hardCodedSurfaceCount = (componentSource.match(/(?:bg|border)-(?:gray|slate|zinc|neutral)-(?:700|800|900)(?:\/[0-9]+)?|(?:bg|border)-\[#[0-9a-fA-F]{3,8}(?:\]\/[0-9]+|\])/g) || []).length;
const defaultPaletteUtilityCount = (componentSource.match(/\b(?:bg|text|border|ring|divide)-(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|white|black)(?:-[0-9]+)?(?:\/[0-9]+)?\b/g) || []).length;
const hardCodedThemeColorCount = (themeCssOutsideFoundation.match(/#[0-9a-fA-F]{3,8}\b|(?:rgb|hsl)a?\(/g) || []).length;
const legacyFieldRadiusCount = [...componentSource.matchAll(/className=(?:"([^"]*)"|`([^`]*)`)/g)]
  .map((match) => match[1] ?? match[2] ?? '')
  .filter((className) => className.includes('theme-input') && className.includes('rounded-xl'))
  .length;
const rawPillRadiusCount = (themeCssOutsideFoundation.match(/border-radius:\s*(?:999|9999)(?:px)?\s*;/g) || []).length;
const semanticRadiusOverrideCount = [...componentSource.matchAll(/className=(?:"([^"]*)"|`([^`]*)`)/g)]
  .map((match) => match[1] ?? match[2] ?? '')
  .filter((className) => /\b(?:clip-card|theme-card-idle|app-dialog-panel|bin-modal-card|settings-panel)\b/.test(className)
    && /\brounded-(?:none|sm|md|lg|xl|2xl|3xl|full|\[[^\]]+\])\b/.test(className))
  .length;
const unthemedDividerCount = [...componentSource.matchAll(/className="([^"]*\bdivide-[xy]\b[^"]*)"/g)]
  .filter((match) => !match[1].includes('theme-divide'))
  .length;
const invalidSemanticButtonCount = (componentSource.match(/\btheme-button-(?:primary|secondary|danger)\b/g) || []).length;

console.log('\nCSS architecture audit');
console.log('----------------------');
console.log(`!important declarations: ${importantCount}/${IMPORTANT_BUDGET}`);
console.log(`Shared light-theme selectors: ${compatibilitySelectorCount}/${COMPATIBILITY_SELECTOR_BUDGET}`);
console.log(`Utility-coupled selectors: ${utilityCoupledSelectorCount}/${UTILITY_COUPLED_SELECTOR_BUDGET}`);
console.log(`Hard-coded JSX surfaces: ${hardCodedSurfaceCount}/${HARD_CODED_SURFACE_BUDGET}`);
console.log(`Default-palette utility debt: ${defaultPaletteUtilityCount}/${DEFAULT_PALETTE_UTILITY_BUDGET}`);
console.log(`Hard-coded theme colors outside foundation: ${hardCodedThemeColorCount}/${HARD_CODED_THEME_COLOR_BUDGET}`);
console.log(`Legacy field radii: ${legacyFieldRadiusCount}/${LEGACY_FIELD_RADIUS_BUDGET}`);
console.log(`Raw pill radii: ${rawPillRadiusCount}/${RAW_PILL_RADIUS_BUDGET}`);
console.log(`Conflicting semantic radius overrides: ${semanticRadiusOverrideCount}/${SEMANTIC_RADIUS_OVERRIDE_BUDGET}`);
console.log(`Unthemed dividers: ${unthemedDividerCount}/0`);
console.log(`Invalid semantic button class names: ${invalidSemanticButtonCount}/0`);
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

if (defaultPaletteUtilityCount > DEFAULT_PALETTE_UTILITY_BUDGET) {
  failed = true;
  console.error(`Default-palette utility budget exceeded by ${defaultPaletteUtilityCount - DEFAULT_PALETTE_UTILITY_BUDGET}. Use semantic theme classes for visual styling.`);
}

if (hardCodedThemeColorCount > HARD_CODED_THEME_COLOR_BUDGET) {
  failed = true;
  console.error('Theme colors must be defined in foundation.css and consumed through semantic custom properties.');
}

if (legacyFieldRadiusCount > LEGACY_FIELD_RADIUS_BUDGET) {
  failed = true;
  console.error('Standard theme inputs must use ui-field-radius instead of a Tailwind radius utility.');
}

if (rawPillRadiusCount > RAW_PILL_RADIUS_BUDGET) {
  failed = true;
  console.error('Pill shapes must use the shared --radius-pill token.');
}

if (semanticRadiusOverrideCount > SEMANTIC_RADIUS_OVERRIDE_BUDGET) {
  failed = true;
  console.error('Semantic card and panel classes already own their radius; remove conflicting Tailwind radius utilities.');
}

if (unthemedDividerCount > 0) {
  failed = true;
  console.error('Every divide-x/divide-y utility must be paired with theme-divide.');
}

if (invalidSemanticButtonCount > 0) {
  failed = true;
  console.error('Use the defined theme-primary-button/theme-secondary-button or app-dialog-button variants.');
}

if (failed) process.exit(1);
console.log('CSS architecture audit passed.');
