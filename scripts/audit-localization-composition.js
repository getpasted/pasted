import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import ts from 'typescript';

const SOURCE_ROOT = 'src';
const ALLOWED_COMPOSITIONS = [
  { file: 'components/ClipRevisionHistory.tsx', snippets: ['LoaderCircle', "translate('component.clipRevisionHistory.loadingOlder')"], reason: 'Loading icon and one complete status label.' },
  { file: 'components/OperationsManager.tsx', snippets: ['operationCategoryLabel', "translate('component.operationsManager.builtIn')"], reason: 'Explicit category and provenance metadata.' },
  { file: 'components/SettingsGeneralPanel.tsx', snippets: ["translate('component.settingsGeneralPanel.colorScheme')", 'appearanceModes.find'], reason: 'Explicit setting label and selected value.' },
  { file: 'components/SettingsSyncPanel.tsx', snippets: ['importInspection.sizeBytes', "translate('component.settingsSyncPanel.completeRecoveryBackup')"], reason: 'Compact type and size metadata.' },
  { file: 'components/SettingsSyncPanel.tsx', snippets: ["'component.settingsSyncPanel.activityInspectionSummary'"], reason: 'Mutually exclusive complete inspection summaries.' },
  { file: 'components/SettingsSyncPanel.tsx', snippets: ["translate('component.settingsSyncPanel.addsNewClipsSkipsExistingMatchesAndKeepsUnrelatedData')"], reason: 'Mutually exclusive complete import descriptions.' },
  { file: 'components/WelcomeSetup.tsx', snippets: ['<Check />', "translate('component.welcomeSetup.ready')"], reason: 'Status icon and one complete label.' },
  { file: 'components/WelcomeSetup.tsx', snippets: ['<Monitor />', "translate('component.welcomeSetup.interface')"], reason: 'Route icon and one complete label.' },
  { file: 'components/WelcomeSetup.tsx', snippets: ['<TerminalSquare />', "translate('component.welcomeSetup.cli')"], reason: 'Route icon and one complete label.' },
  { file: 'components/WelcomeSetup.tsx', snippets: ['<Workflow />', "translate('component.welcomeSetup.automations')"], reason: 'Route icon and one complete label.' },
  { file: 'components/WelcomeSetup.tsx', snippets: ['<Bot />', "translate('component.welcomeSetup.agents')"], reason: 'Route icon and one complete label.' },
];

const files = [];
function walk(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory()) walk(fullPath);
    else if (/\.tsx?$/.test(entry.name)) files.push(fullPath);
  }
}
walk(SOURCE_ROOT);

const isTranslateCall = (node) => ts.isCallExpression(node)
  && ts.isIdentifier(node.expression)
  && node.expression.text === 'translate';

function contains(node, predicate) {
  let found = false;
  function visit(child) {
    if (predicate(child)) found = true;
    if (!found) ts.forEachChild(child, visit);
  }
  visit(node);
  return found;
}

const containsTranslate = (node) => contains(node, isTranslateCall);
const containsJsx = (node) => contains(node, (child) => ts.isJsxElement(child) || ts.isJsxSelfClosingElement(child));
const candidates = [];

for (const file of files) {
  const source = fs.readFileSync(file, 'utf8');
  const sourceFile = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, file.endsWith('.tsx') ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  function addCandidate(kind, node) {
    candidates.push({
      file: path.relative(SOURCE_ROOT, file),
      line: sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1,
      kind,
      source: node.getText(sourceFile).replace(/\s+/g, ' '),
    });
  }
  function visit(node) {
    const isCompositionExpression = ts.isTemplateExpression(node)
      || (ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.PlusToken);
    if (isCompositionExpression && containsTranslate(node)) {
      const parentIsComposition = (ts.isTemplateExpression(node.parent)
        || (ts.isBinaryExpression(node.parent) && node.parent.operatorToken.kind === ts.SyntaxKind.PlusToken))
        && containsTranslate(node.parent);
      if (!parentIsComposition) addCandidate('expression', node);
    }

    if (ts.isJsxElement(node) || ts.isJsxFragment(node)) {
      const parentTag = ts.isJsxElement(node) ? node.openingElement.tagName.getText(sourceFile) : '';
      if (!parentTag || /^(?:p|span|li|label|dd|dt)$/.test(parentTag)) {
        const parts = [];
        for (const child of node.children) {
          if (ts.isJsxText(child)) {
            if (child.getText(sourceFile).trim()) parts.push('literal');
          } else if (ts.isJsxExpression(child) && child.expression) {
            if (!containsJsx(child.expression) && containsTranslate(child.expression)) parts.push('translated');
            else if (!containsJsx(child.expression)) parts.push('dynamic');
          } else if (ts.isJsxElement(child)) {
            const tag = child.openingElement.tagName.getText(sourceFile);
            if (/^(?:code|kbd|strong|em|span)$/.test(tag)) parts.push('inline');
          } else if (ts.isJsxSelfClosingElement(child)) {
            parts.push('inline');
          }
        }
        if (parts.includes('translated') && parts.length > 1) addCandidate(`jsx:${parts.join(',')}`, node);
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(sourceFile);
}

const matchedExceptions = new Set();
if (process.env.DEBUG_LOCALIZATION_COMPOSITION === '1') {
  for (const candidate of candidates) console.log(JSON.stringify(candidate));
}
const violations = candidates.filter((candidate) => {
  const matchingIndexes = ALLOWED_COMPOSITIONS.flatMap(({ file, snippets }, index) => (
    file === candidate.file && snippets.every((snippet) => candidate.source.includes(snippet)) ? [index] : []
  ));
  for (const index of matchingIndexes) matchedExceptions.add(index);
  return matchingIndexes.length === 0;
});

for (const [index, exception] of ALLOWED_COMPOSITIONS.entries()) {
  assert.ok(matchedExceptions.has(index), `Stale localization-composition exception: ${exception.file} (${exception.reason})`);
}
assert.deepEqual(
  violations,
  [],
  `Translated sentences must use one complete message with placeholders or an explicit label/value layout:\n${violations.map(({ file, line, source }) => `${file}:${line}: ${source}`).join('\n')}`,
);

console.log(`Localization composition audit passed with ${ALLOWED_COMPOSITIONS.length} documented structural exceptions.`);
