import assert from 'node:assert/strict';
import fs from 'node:fs';

const [sbomPath, platform] = process.argv.slice(2);
assert.ok(sbomPath && platform, 'Usage: node scripts/audit-artifact-sbom.js SBOM_PATH PLATFORM');
const policy = JSON.parse(fs.readFileSync('dependency-policy.json', 'utf8'));
const sbom = JSON.parse(fs.readFileSync(sbomPath, 'utf8'));
const packages = (sbom.packages ?? []).filter((component) => (
  component.primaryPackagePurpose !== 'FILE' || component.versionInfo
));
const files = sbom.files ?? [];
assert.ok(files.length > 0, `Artifact SBOM for ${platform} did not inventory any files`);
const fileNames = files.map(({ fileName }) => fileName.replaceAll('\\', '/'));
for (const requiredPattern of policy.artifactRequiredFilePatterns[platform] ?? []) {
  const pattern = new RegExp(requiredPattern, 'i');
  assert.ok(
    fileNames.some((fileName) => pattern.test(fileName)),
    `${platform} artifact SBOM is missing required payload evidence: ${requiredPattern}`,
  );
}
const allowedUnknown = new Set([
  ...(policy.artifactUnknownLicenseAllowlist[platform] ?? []),
  ...policy.packagingComponents.flatMap((component) => component.artifactUnknownNames[platform] ?? []),
]);
const allowedUnknownPatterns = (policy.artifactUnknownLicensePatterns[platform] ?? []).map((pattern) => new RegExp(pattern));
const forbidden = new RegExp(policy.forbiddenLicenseTerms.join('|'), 'i');
for (const component of packages) {
  const expression = component.licenseConcluded ?? component.licenseDeclared ?? 'NOASSERTION';
  assert.doesNotMatch(expression, forbidden, `${platform} artifact contains forbidden license: ${component.name} [${expression}]`);
  if (expression === 'NOASSERTION' || expression === 'NONE') {
    assert.ok(
      allowedUnknown.has(component.name) || allowedUnknownPatterns.some((pattern) => pattern.test(component.name)),
      `${platform} artifact has an unreviewed unknown license: ${component.name}`,
    );
  }
}
console.log(
  `Artifact SBOM audit passed for ${files.length} ${platform} files and `
  + `${packages.length} detected package records.`,
);
