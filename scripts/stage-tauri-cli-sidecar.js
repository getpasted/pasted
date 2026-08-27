import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

export function executableExtension(platform) {
  return platform === 'win32' ? '.exe' : '';
}

export function sidecarFilename(targetTriple, platform) {
  return `pasted-${targetTriple}${executableExtension(platform)}`;
}

export function stageCliSidecar({
  root = projectRoot,
  targetDir = process.env.CARGO_TARGET_DIR
    ? path.resolve(process.cwd(), process.env.CARGO_TARGET_DIR)
    : path.join(root, 'src-tauri', 'target'),
  targetTriple = execFileSync('rustc', ['--print', 'host-tuple'], { encoding: 'utf8' }).trim(),
  platform = process.platform,
} = {}) {
  if (!targetTriple) {
    throw new Error('rustc did not report a target triple');
  }

  const extension = executableExtension(platform);
  const source = path.join(targetDir, 'release', `pasted${extension}`);
  if (!fs.existsSync(source)) {
    throw new Error(`Headless CLI has not been built: ${source}`);
  }

  const destinationDirectory = path.join(root, 'src-tauri', 'binaries');
  const destination = path.join(destinationDirectory, sidecarFilename(targetTriple, platform));
  fs.mkdirSync(destinationDirectory, { recursive: true });
  fs.copyFileSync(source, destination);
  if (platform !== 'win32') {
    fs.chmodSync(destination, fs.statSync(source).mode);
  }
  return { source, destination, targetTriple };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const argumentValue = (name) => {
    const index = process.argv.indexOf(name);
    return index >= 0 ? process.argv[index + 1] : undefined;
  };
  const argumentValues = (name) => process.argv.flatMap((argument, index) => (
    argument === name && process.argv[index + 1] ? [process.argv[index + 1]] : []
  ));
  const targetDir = argumentValue('--target-dir');
  const targetTriples = argumentValues('--target-triple');
  for (const targetTriple of targetTriples.length > 0 ? targetTriples : [undefined]) {
    const staged = stageCliSidecar({
      targetDir: targetDir ? path.resolve(process.cwd(), targetDir) : undefined,
      targetTriple,
      platform: argumentValue('--platform'),
    });
    console.log(`Staged ${staged.source} for Tauri as ${staged.destination}`);
  }
}
