import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';

const outputPath = 'THIRD_PARTY_SBOM.spdx.json';
const checkOnly = process.argv.includes('--check');
const inventory = JSON.parse(fs.readFileSync('THIRD_PARTY_LICENSES.json', 'utf8'));
const packageJson = JSON.parse(fs.readFileSync('package.json', 'utf8'));
const digest = crypto.createHash('sha256').update(JSON.stringify(inventory.components)).digest('hex');
const spdxId = (component) => `SPDXRef-${component.ecosystem}-${crypto.createHash('sha256').update(`${component.name}@${component.version}`).digest('hex').slice(0, 16)}`;
const purlName = (value) => encodeURIComponent(value).replaceAll('%2F', '/');
const purlType = (ecosystem) => ecosystem === 'packaging' ? 'generic' : ecosystem;
const packages = inventory.components.map((component) => ({
  SPDXID: spdxId(component),
  name: component.name,
  versionInfo: component.version,
  downloadLocation: component.repository || 'NOASSERTION',
  filesAnalyzed: false,
  licenseConcluded: component.license,
  licenseDeclared: component.license,
  copyrightText: 'NOASSERTION',
  externalRefs: [{
    referenceCategory: 'PACKAGE-MANAGER',
    referenceType: 'purl',
    referenceLocator: `pkg:${purlType(component.ecosystem)}/${purlName(component.name)}@${component.version}`,
  }],
}));
const document = {
  spdxVersion: 'SPDX-2.3',
  dataLicense: 'CC0-1.0',
  SPDXID: 'SPDXRef-DOCUMENT',
  name: `Pasted-${packageJson.version}-distributed-components`,
  documentNamespace: `https://getpasted.app/sbom/${packageJson.version}/${digest}`,
  creationInfo: {
    creators: ['Tool: Pasted deterministic dependency inventory'],
    created: '1970-01-01T00:00:00Z'
  },
  packages,
  relationships: packages.map((component) => ({
    spdxElementId: 'SPDXRef-DOCUMENT',
    relationshipType: 'DESCRIBES',
    relatedSpdxElement: component.SPDXID,
  })),
};
const serialized = `${JSON.stringify(document, null, 2)}\n`;
if (checkOnly) {
  assert.equal(fs.readFileSync(outputPath, 'utf8'), serialized, 'Source SBOM is stale; run npm run sbom:generate');
  console.log(`Source SBOM audit passed for ${packages.length} components.`);
} else {
  fs.writeFileSync(outputPath, serialized);
  console.log(`Generated ${outputPath} for ${packages.length} components.`);
}
