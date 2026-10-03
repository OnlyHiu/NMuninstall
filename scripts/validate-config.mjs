// Validates src-tauri/tauri.conf.json against the CLI's bundled JSON schema.
// Catches config typos without paying for a full `cargo check` cycle.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const schema = JSON.parse(
  readFileSync(resolve(root, 'node_modules/@tauri-apps/cli/config.schema.json'), 'utf8'),
);
const config = JSON.parse(readFileSync(resolve(root, 'src-tauri/tauri.conf.json'), 'utf8'));

const errors = [];

function ref(name, path) {
  // `$ref` looks like `#/definitions/AppConfig`; walk it segment by segment.
  const target = name
    .replace(/^#\//, '')
    .split('/')
    .reduce((node, seg) => node?.[seg], schema);
  if (!target) errors.push(`${path}: schema ${name} is missing from config.schema.json`);
  return target;
}

function check(node, sch, path) {
  if (typeof node !== 'object' || node === null) return;
  if (sch === true || sch === undefined) return;

  if (sch.$ref) {
    const next = ref(sch.$ref, path);
    if (!next) return;
    return check(node, next, path);
  }
  if (sch.allOf) {
    for (const sub of sch.allOf) check(node, sub, path);
    return;
  }
  if (sch.anyOf) {
    if (!sch.anyOf.some((s) => matches(node, s))) {
      errors.push(`${path}: no variant of anyOf matched`);
    }
    return;
  }
  if (sch.oneOf) {
    if (!sch.oneOf.some((s) => matches(node, s))) {
      errors.push(`${path}: no variant of oneOf matched`);
    }
    return;
  }
  if (sch.enum && !sch.enum.includes(node)) {
    errors.push(`${path}: "${node}" is not one of ${sch.enum.join(', ')}`);
    return;
  }

  if (Array.isArray(node)) {
    if (sch.items) node.forEach((v, i) => check(v, sch.items, `${path}[${i}]`));
    return;
  }

  const props = sch.properties;
  if (props) {
    for (const key of Object.keys(node)) {
      if (key in props) {
        check(node[key], props[key], `${path}.${key}`);
      } else if (sch.additionalProperties === false) {
        errors.push(`${path}.${key}: unknown field`);
      } else if (typeof sch.additionalProperties === 'object') {
        check(node[key], sch.additionalProperties, `${path}.${key}`);
      }
    }
    for (const req of sch.required ?? []) {
      if (!(req in node)) errors.push(`${path}: missing required field "${req}"`);
    }
  }
}

function matches(node, sch) {
  if (sch.$ref) sch = ref(sch.$ref);
  if (sch.anyOf || sch.oneOf) {
    const list = sch.anyOf ?? sch.oneOf;
    return list.some((s) => matches(node, s));
  }
  if (sch.enum) return sch.enum.includes(node);
  return true;
}

check(config, schema, 'tauri.conf.json');

if (errors.length === 0) {
  console.log('tauri.conf.json: OK');
} else {
  console.error(`tauri.conf.json: ${errors.length} problem(s)`);
  for (const e of errors) console.error(`  - ${e}`);
  process.exit(1);
}
