import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

const require = createRequire(import.meta.url);
const ts = require('typescript-compiler-api');

const roots = ['src/App.tsx', 'src/components', 'src/hooks/useAppLock.ts', 'src/types.ts'];
const files = [];
const collect = (entry) => {
  const stat = fs.statSync(entry);
  if (stat.isDirectory()) {
    for (const child of fs.readdirSync(entry)) collect(path.join(entry, child));
  } else if (/\.tsx?$/.test(entry)) files.push(entry);
};
roots.forEach(collect);

const userFacingNames = /^(?:alt|title|label|.*Label|description|details|message|.*Message|placeholder|.*Placeholder|emptyTitle|emptyDescription|subtitle|summary|createLabel)$/i;
const userFacingSetters = /^set(?:Error|LoadError|TestOutput|Message|Summary)$/;
const technicalValues = new Set([
  'HEX', 'RGB', 'HSL', 'Whisper.cpp', 'MediaInfo', 'ffprobe',
]);
const isTechnical = (value) => technicalValues.has(value)
  || /^\/?(?:usr|path|tmp)\//.test(value)
  || /^(?:https?:\/\/|[A-Z][A-Z0-9_]+$)/.test(value)
  || /^e\.g\. /.test(value)
  || /^(?:--|pasted(?:\s|$)|\.pastedbackup$|custom-command-v1$|⌥ |text$)/.test(value)
  || /&(?:lt|gt|quot);/.test(value)
  || /^enable[A-Z]/.test(value)
  || /^[a-z][A-Za-z0-9]*(?:\.[a-zA-Z][A-Za-z0-9]*)+$/.test(value);
const isCss = (value) => /(?:^|\s)(?:@|\[|flex|grid|text-|theme-|is-|cursor-|opacity-|shadow|border|rounded|hover:|active:|pointer|animate|justify|max-|min-|space-|py-|px-|p-|m[trblxy]?-|rotate|overflow|invisible|visible|w-|h-|gap-|col-|row-)/.test(value);
const jsxExpressionContext = (node) => {
  for (let current = node.parent; current && !ts.isStatement(current); current = current.parent) {
    if (ts.isJsxExpression(current)) {
      return ts.isJsxAttribute(current.parent) ? current.parent.name.getText() : 'child';
    }
  }
  return null;
};

const findings = [];
for (const file of files) {
  const source = fs.readFileSync(file, 'utf8');
  const sourceFile = ts.createSourceFile(
    file,
    source,
    ts.ScriptTarget.Latest,
    true,
    file.endsWith('.tsx') ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  const record = (node, context, value) => {
    if (!/[A-Za-z]{2}/.test(value) || isTechnical(value) || isCss(value)) return;
    const line = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;
    findings.push(`${file}:${line} ${context}: ${JSON.stringify(value)}`);
  };
  const visit = (node) => {
    if (ts.isJsxText(node) && /[A-Za-z]{2}/.test(node.text.trim())) {
      record(node, 'JSX text', node.text.trim());
    }
    if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
      const value = node.text;
      const parent = node.parent;
      if (ts.isImportDeclaration(parent) || ts.isLiteralTypeNode(parent)) {
        ts.forEachChild(node, visit);
        return;
      }
      if (ts.isJsxAttribute(parent) && userFacingNames.test(parent.name.getText(sourceFile))) {
        record(node, `JSX ${parent.name.getText(sourceFile)}`, value);
      } else if (ts.isPropertyAssignment(parent) && userFacingNames.test(parent.name.getText(sourceFile).replace(/["']/g, ''))) {
        record(node, `property ${parent.name.getText(sourceFile)}`, value);
      } else if (ts.isBindingElement(parent) && parent.initializer === node && userFacingNames.test(parent.name.getText(sourceFile))) {
        record(node, `default ${parent.name.getText(sourceFile)}`, value);
      } else if (ts.isCallExpression(parent) && parent.arguments.includes(node) && userFacingSetters.test(parent.expression.getText(sourceFile))) {
        record(node, `call ${parent.expression.getText(sourceFile)}`, value);
      } else if (ts.isConditionalExpression(parent) && (parent.whenTrue === node || parent.whenFalse === node)) {
        const context = jsxExpressionContext(parent);
        if (context === 'child' || (context && userFacingNames.test(context))) record(node, 'JSX conditional', value);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
}

assert.deepEqual(findings, [], `User-facing source literals must use localization keys:\n${findings.join('\n')}`);
console.log(`Syntax-aware localization literal audit passed across ${files.length} UI source files.`);
