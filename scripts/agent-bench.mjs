#!/usr/bin/env node
/**
 * ChefBar agent-harness benchmark.
 *
 * Deterministic (no LLM, no network). Scores:
 *   1. structure  — frontmatter, required sections, name/path match
 *   2. graph      — workers ↔ graph.yaml ↔ skills
 *   3. routing    — trigger prompts rank the expected skill/agent first
 *   4. invariants — Cargo.toml (no tokio/reqwest), CSS GTK3-subset, disjoint owns
 *   5. evals      — per-skill evals/evals.json schema + description coverage
 *
 * Usage:
 *   node scripts/agent-bench.mjs
 *   node scripts/agent-bench.mjs --json
 *   node scripts/agent-bench.mjs --min-routing 0.75
 *
 * Exit 0 if all blocking checks pass and routing >= --min-routing (default 0.75).
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..");
const CURSOR = path.join(ROOT, ".cursor");
const args = new Set(process.argv.slice(2));
const jsonOnly = args.has("--json");
const minRouting = (() => {
  const i = process.argv.indexOf("--min-routing");
  if (i < 0) return 0.75;
  const raw = process.argv[i + 1];
  const n = Number(raw);
  if (raw === undefined || !Number.isFinite(n) || n < 0 || n > 1) {
    console.error(
      `invalid --min-routing ${raw === undefined ? "(missing)" : JSON.stringify(raw)}; expected a number in [0, 1]`,
    );
    process.exit(1);
  }
  return n;
})();

const STOP = new Set(
  `a an the to for of in on at as is it or and when use whenever also this that
   de het een van en in op te voor met als bij om uit tot dat die dit deze
   skill skills agent agents chefbar worker workers`.split(/\s+/),
);

function walk(dir, pred) {
  if (!fs.existsSync(dir)) return [];
  const out = [];
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, ent.name);
    if (ent.isDirectory()) out.push(...walk(p, pred));
    else if (pred(p)) out.push(p);
  }
  return out;
}

function parseFrontmatter(text, file) {
  if (!text.startsWith("---\n") && !text.startsWith("---\r\n")) {
    return { error: `${file}: missing opening ---` };
  }
  const rest = text.replace(/^---\r?\n/, "");
  const end = rest.search(/\r?\n---\r?\n/);
  if (end < 0) return { error: `${file}: missing closing ---` };
  const raw = rest.slice(0, end);
  const body = rest.slice(end).replace(/^\r?\n---\r?\n/, "");
  const data = {};
  let key = null;
  let buf = [];
  const flush = () => {
    if (!key) return;
    data[key] = buf.join("\n").trim();
    key = null;
    buf = [];
  };
  for (const line of raw.split(/\r?\n/)) {
    const m = line.match(/^([A-Za-z0-9_-]+):\s*(.*)$/);
    if (m && !line.startsWith(" ")) {
      flush();
      key = m[1];
      const v = m[2];
      if (v === "|" || v === ">" || v === "|-" || v === ">-" || v === "|+" || v === ">+") {
        buf = [];
      } else {
        data[key] = v.replace(/^["']|["']$/g, "");
        key = null;
      }
    } else if (key) {
      buf.push(line.replace(/^\s{2}/, ""));
    }
  }
  flush();
  return { data, body, raw };
}

function tokens(s) {
  return String(s || "")
    .toLowerCase()
    .split(/[^a-z0-9_./+-]+/)
    .filter((t) => t.length >= 2 && !STOP.has(t));
}

function overlap(query, doc) {
  const q = new Set(tokens(query));
  const d = tokens(doc);
  if (q.size === 0 || d.length === 0) return 0;
  const dc = new Map();
  for (const t of d) dc.set(t, (dc.get(t) || 0) + 1);
  let score = 0;
  for (const t of q) {
    if (dc.has(t)) score += 1 + Math.log(1 + dc.get(t));
  }
  // filename-like tokens weigh more
  for (const t of q) {
    if (t.includes(".rs") || t.includes("/")) {
      if (dc.has(t)) score += 3;
    }
  }
  return score;
}

function escapeRegExp(s) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function headingStartsWithName(heading, name) {
  const h = heading.trim().toLowerCase();
  const n = name.toLowerCase();
  const re = new RegExp(`^${escapeRegExp(n)}s?(?:[^a-z0-9]|$)`);
  return re.test(h);
}

/** Cursor skill/agent loaders ignore YAML `>` / `|` folds — description must be one line. */
function isFoldedDescription(raw) {
  return /^description:\s*[>|][+-]?\s*$/m.test(raw || "");
}

/** Stateless harness: rules must not auto-attach. */
function isAlwaysApplyTrue(raw) {
  return /^alwaysApply:\s*true(?:\s+#.*)?\s*$/im.test(raw || "");
}

function hasGlobsKey(raw) {
  return /^globs\s*:/m.test(raw || "");
}

function hasDisableModelInvocation(raw) {
  return /^disable-model-invocation:\s*true(?:\s+#.*)?\s*$/im.test(raw || "");
}

function hasHeading(body, names) {
  const heads = [...body.matchAll(/^#{1,3}\s+(.+)$/gm)].map((m) =>
    m[1].trim().toLowerCase(),
  );
  return names.some((n) => heads.some((h) => headingStartsWithName(h, n)));
}

function rel(p) {
  return path.relative(ROOT, p);
}

function loadJson(p) {
  return JSON.parse(fs.readFileSync(p, "utf8"));
}

const report = {
  generated_at: new Date().toISOString(),
  blocking: [],
  warnings: [],
  skills: [],
  agents: [],
  commands: [],
  rules: [],
  routing: { cases: [], accuracy: 0, passed: 0, total: 0 },
  invariants: {},
  evals: { skills_with_evals: 0, skills_missing_evals: [] },
  score: { structure: 0, routing: 0, quality: 0, overall: 0 },
};

function fail(msg) {
  report.blocking.push(msg);
}
function warn(msg) {
  report.warnings.push(msg);
}

function readTextOrFail(p, label) {
  if (!fs.existsSync(p)) {
    fail(`${label} missing`);
    return "";
  }
  return fs.readFileSync(p, "utf8");
}

{
  const headingCases = [
    ["output", "Output", true],
    ["output", "Output notes for done items", true],
    ["done", "Output notes for done items", false],
    ["done", "Definition of done", false],
    ["definition", "Definition of done", true],
    ["example", "Examples", true],
    ["anti-pattern", "Anti-patterns", true],
    ["performance", "Performance Notes", true],
  ];
  for (const [name, heading, expect] of headingCases) {
    const got = hasHeading(`## ${heading}\n`, [name]);
    if (got !== expect) {
      fail(`hasHeading('${name}', '${heading}') expected ${expect}, got ${got}`);
    }
  }
  const foldCases = [
    ["description: one line\n", false],
    ["description: >-\n  folded\n", true],
    ["description: >\n  folded\n", true],
    ["description: |\n  literal\n", true],
    ["name: x\ndescription: Use when editing src/state.rs.\n", false],
  ];
  for (const [raw, expect] of foldCases) {
    const got = isFoldedDescription(raw);
    if (got !== expect) {
      fail(`isFoldedDescription(${JSON.stringify(raw)}) expected ${expect}, got ${got}`);
    }
  }
  const alwaysCases = [
    ["alwaysApply: true\n", true],
    ["alwaysApply: true # stateless rules load via chain\n", true],
    ["alwaysApply: false\n", false],
    ["alwaysApply: True\n", true],
    ["description: x\n", false],
  ];
  for (const [raw, expect] of alwaysCases) {
    const got = isAlwaysApplyTrue(raw);
    if (got !== expect) {
      fail(`isAlwaysApplyTrue(${JSON.stringify(raw)}) expected ${expect}, got ${got}`);
    }
  }
  const globCases = [
    ['globs: "**/*.rs"\n', true],
    ["globs:\n  - src/css.rs\n", true],
    ["alwaysApply: false\n", false],
  ];
  for (const [raw, expect] of globCases) {
    const got = hasGlobsKey(raw);
    if (got !== expect) {
      fail(`hasGlobsKey(${JSON.stringify(raw)}) expected ${expect}, got ${got}`);
    }
  }
  const dmiCases = [
    ["disable-model-invocation: true\n", true],
    ["disable-model-invocation: true # ambient off\n", true],
    ["disable-model-invocation: false\n", false],
    ["description: x\n", false],
  ];
  for (const [raw, expect] of dmiCases) {
    const got = hasDisableModelInvocation(raw);
    if (got !== expect) {
      fail(`hasDisableModelInvocation(${JSON.stringify(raw)}) expected ${expect}, got ${got}`);
    }
  }
}

// --- skills ---
const skillsRoot = path.join(CURSOR, "skills");
const skillDirs = fs.existsSync(skillsRoot)
  ? fs
      .readdirSync(skillsRoot, { withFileTypes: true })
      .filter((e) => e.isDirectory())
      .map((e) => path.join(skillsRoot, e.name))
  : (fail(".cursor/skills missing"), []);

for (const dir of skillDirs) {
  const name = path.basename(dir);
  const skillMd = path.join(dir, "SKILL.md");
  const rec = { name, path: rel(skillMd), ok: true, issues: [], quality: [] };
  if (!fs.existsSync(skillMd)) {
    rec.ok = false;
    rec.issues.push("SKILL.md missing");
    fail(`skill ${name}: SKILL.md missing`);
    report.skills.push(rec);
    continue;
  }
  const text = fs.readFileSync(skillMd, "utf8");
  const fm = parseFrontmatter(text, rel(skillMd));
  if (fm.error) {
    rec.ok = false;
    rec.issues.push(fm.error);
    fail(fm.error);
    report.skills.push(rec);
    continue;
  }
  const desc = fm.data.description || "";
  const nm = fm.data.name || "";
  rec.description_chars = desc.length;
  rec.body_lines = fm.body.split(/\n/).length;
  if (nm !== name) {
    rec.ok = false;
    rec.issues.push(`frontmatter name '${nm}' != directory '${name}'`);
    fail(`skill ${name}: name mismatch`);
  }
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(nm)) {
    rec.ok = false;
    rec.issues.push("name not kebab-case");
    fail(`skill ${name}: name not kebab-case`);
  }
  if (isFoldedDescription(fm.raw)) {
    rec.ok = false;
    rec.issues.push("description uses YAML > / | fold");
    fail(
      `skill ${name}: description must be one physical line (Cursor does not parse >- folds)`,
    );
  }
  if (hasDisableModelInvocation(fm.raw)) {
    rec.ok = false;
    rec.issues.push("disable-model-invocation: true");
    fail(
      `skill ${name}: disable-model-invocation kills description triggers; harness is stateless`,
    );
  }
  if (!desc) {
    rec.ok = false;
    rec.issues.push("empty description");
    fail(`skill ${name}: empty description`);
  }
  if (desc.includes("<") || desc.includes(">")) {
    rec.ok = false;
    rec.issues.push("description contains angle brackets");
    fail(`skill ${name}: description has <>`);
  }
  if (desc.length > 1024) {
    rec.ok = false;
    rec.issues.push(`description too long (${desc.length})`);
    fail(`skill ${name}: description > 1024`);
  }
  if (desc.length < 120) {
    rec.issues.push("description short (<120)");
    warn(`skill ${name}: description is thin (${desc.length} chars)`);
  }
  if (rec.body_lines > 500) {
    warn(`skill ${name}: SKILL.md is ${rec.body_lines} lines (prefer <500 + references/)`);
  }
  const requiredHeads = [
    ["instructions", "playbook", "workflow"],
    ["example"],
    ["performance"],
    ["troubleshooting"],
  ];
  for (const group of requiredHeads) {
    if (!hasHeading(fm.body, group)) {
      rec.ok = false;
      rec.issues.push(`missing heading matching ${group.join("|")}`);
      fail(`skill ${name}: missing section (${group[0]})`);
    }
  }
  rec.index = `${nm} ${desc} ${fm.body.slice(0, 4000)}`;
  rec.description = desc;

  const evalsPath = path.join(dir, "evals", "evals.json");
  const trigPath = path.join(dir, "evals", "triggers.json");
  rec.has_evals = fs.existsSync(evalsPath);
  rec.has_triggers = fs.existsSync(trigPath);
  if (rec.has_evals) {
    report.evals.skills_with_evals += 1;
    try {
      const ev = loadJson(evalsPath);
      if (ev.skill_name !== name) {
        rec.issues.push(`evals.skill_name '${ev.skill_name}' != ${name}`);
        fail(`skill ${name}: evals skill_name mismatch`);
      }
      if (!Array.isArray(ev.evals) || ev.evals.length < 3) {
        rec.issues.push("evals.json needs >= 3 cases");
        fail(`skill ${name}: fewer than 3 evals`);
      } else {
        rec.eval_count = ev.evals.length;
        for (const e of ev.evals) {
          if (!e.prompt || !e.expected_output) {
            rec.issues.push(`eval ${e.id} missing prompt/expected_output`);
            fail(`skill ${name}: eval ${e.id} incomplete`);
          }
          if (!Array.isArray(e.expectations) || e.expectations.length < 2) {
            rec.issues.push(`eval ${e.id} needs >= 2 expectations`);
            fail(`skill ${name}: eval ${e.id} thin expectations`);
          }
        }
      }
    } catch (err) {
      rec.ok = false;
      rec.issues.push(`evals.json parse: ${err.message}`);
      fail(`skill ${name}: evals.json parse error`);
    }
  } else {
    report.evals.skills_missing_evals.push(name);
    rec.ok = false;
    rec.issues.push("missing evals/evals.json");
    fail(`skill ${name}: missing evals/evals.json`);
  }
  if (!rec.has_triggers) {
    rec.ok = false;
    rec.issues.push("missing evals/triggers.json");
    fail(`skill ${name}: missing evals/triggers.json`);
  }

  // quality signals
  if (/\.rs\b/.test(desc + fm.body)) rec.quality.push("repo-paths");
  if (/tokio|reqwest|webview|electron/i.test(fm.body)) rec.quality.push("names-forbidden-stack");
  if (fs.existsSync(path.join(dir, "references"))) rec.quality.push("references");
  rec.quality_score = Math.min(
    100,
    (desc.length > 200 ? 20 : 10) +
      rec.quality.length * 15 +
      (rec.eval_count || 0) * 5 +
      (rec.has_triggers ? 15 : 0) +
      (rec.body_lines > 80 ? 15 : 5),
  );

  report.skills.push(rec);
}

// --- agents ---
const agentFiles = walk(path.join(CURSOR, "agents"), (p) => p.endsWith(".md"));
for (const file of agentFiles) {
  const stem = path.basename(file, ".md");
  const text = fs.readFileSync(file, "utf8");
  const fm = parseFrontmatter(text, rel(file));
  const rec = { name: stem, path: rel(file), ok: true, issues: [] };
  if (fm.error) {
    rec.ok = false;
    rec.issues.push(fm.error);
    fail(fm.error);
    report.agents.push(rec);
    continue;
  }
  if ((fm.data.name || "") !== stem) {
    rec.ok = false;
    rec.issues.push("name != filename stem");
    fail(`agent ${stem}: name mismatch`);
  }
  const desc = fm.data.description || "";
  rec.description = desc;
  rec.description_chars = desc.length;
  if (isFoldedDescription(fm.raw)) {
    rec.ok = false;
    rec.issues.push("description uses YAML > / | fold");
    fail(
      `agent ${stem}: description must be one physical line (Cursor does not parse >- folds)`,
    );
  }
  if (desc.length < 120) {
    rec.issues.push("description short");
    warn(`agent ${stem}: description is thin`);
  }
  if (desc.length > 1024) {
    rec.ok = false;
    fail(`agent ${stem}: description > 1024`);
  }
  const groups = [
    ["owns", "identity"],
    ["playbook", "workflow", "instructions"],
    ["output"],
    ["handoff", "anti-pattern"],
    ["done", "definition"],
  ];
  for (const g of groups) {
    if (!hasHeading(fm.body, g)) {
      rec.ok = false;
      rec.issues.push(`missing heading ${g[0]}`);
      fail(`agent ${stem}: missing section (${g[0]})`);
    }
  }
  rec.index = `${stem} ${desc} ${fm.body.slice(0, 4000)}`;
  rec.body_lines = fm.body.split(/\n/).length;
  if (rec.body_lines < 80) {
    warn(`agent ${stem}: body is thin (${rec.body_lines} lines)`);
  }
  report.agents.push(rec);
}

// --- commands ---
for (const file of walk(path.join(CURSOR, "commands"), (p) => p.endsWith(".md"))) {
  const stem = path.basename(file, ".md");
  const text = fs.readFileSync(file, "utf8");
  const fm = parseFrontmatter(text, rel(file));
  const rec = { name: stem, path: rel(file), ok: true };
  if (fm.error || (fm.data.name || "") !== stem || !fm.data.description) {
    rec.ok = false;
    fail(`command ${stem}: invalid frontmatter`);
  }
  report.commands.push(rec);
}

// --- rules (stateless: description trigger only) ---
const rulesRoot = path.join(CURSOR, "rules");
report.rules = [];
const ruleFiles = fs.existsSync(rulesRoot)
  ? walk(rulesRoot, (p) => p.endsWith(".mdc"))
  : (fail(".cursor/rules missing"), []);
for (const file of ruleFiles) {
  const stem = path.basename(file);
  const text = fs.readFileSync(file, "utf8");
  const fm = parseFrontmatter(text, rel(file));
  const rec = { name: stem, path: rel(file), ok: true, issues: [] };
  if (fm.error) {
    rec.ok = false;
    rec.issues.push(fm.error);
    fail(fm.error);
    report.rules.push(rec);
    continue;
  }
  if (isFoldedDescription(fm.raw)) {
    rec.ok = false;
    rec.issues.push("description uses YAML > / | fold");
    fail(
      `rule ${stem}: description must be one physical line (Cursor does not parse >- folds)`,
    );
  }
  if (!(fm.data.description || "").trim()) {
    rec.ok = false;
    rec.issues.push("empty description");
    fail(`rule ${stem}: empty description`);
  }
  if (isAlwaysApplyTrue(fm.raw)) {
    rec.ok = false;
    rec.issues.push("alwaysApply: true");
    fail(
      `rule ${stem}: alwaysApply must not be true (stateless — load via description or chain)`,
    );
  }
  if (hasGlobsKey(fm.raw)) {
    rec.ok = false;
    rec.issues.push("globs key");
    fail(
      `rule ${stem}: no globs (stateless — load via description or chain, not file attach)`,
    );
  }
  report.rules.push(rec);
}

// --- graph ---
const graphPath = path.join(
  CURSOR,
  "skills/chefbar-graph-loop/references/graph.yaml",
);
let graphText = "";
if (fs.existsSync(graphPath)) {
  graphText = fs.readFileSync(graphPath, "utf8");
  const agentIds = [...graphText.matchAll(/agent:\s*([a-z0-9-]+)/g)].map(
    (m) => m[1],
  );
  const unique = [...new Set(agentIds)];
  const have = new Set(report.agents.map((a) => a.name));
  for (const id of unique) {
    if (!have.has(id)) fail(`graph.yaml agent '${id}' has no .cursor/agents file`);
  }
  for (const a of report.agents) {
    if (!unique.includes(a.name) && a.name !== "chefbar-orchestrator") {
      warn(`agent ${a.name} not referenced in graph.yaml`);
    }
  }
} else {
  fail("graph.yaml missing");
}

const expectedPairing = {
  "chefbar-orchestrator": "chefbar-graph-loop",
  "chefbar-architect": "chefbar-architecture",
  "chefbar-rust-core": "chefbar-rust",
  "chefbar-actor": "chefbar-actor",
  "chefbar-gtk-panel": "chefbar-gtk-panel",
  "chefbar-tray-ipc": "chefbar-tray-ipc",
  "chefbar-policy-http": "chefbar-policy-http",
  "chefbar-actions-palette": "chefbar-actions-palette",
  "chefbar-qa": "chefbar-qa",
  "chefbar-kater": "chefbar-kater",
};
const skillNames = new Set(report.skills.map((s) => s.name));
for (const [agent, skill] of Object.entries(expectedPairing)) {
  if (!report.agents.some((a) => a.name === agent)) {
    fail(`expected agent ${agent} missing`);
  }
  if (!skillNames.has(skill)) {
    fail(`expected skill ${skill} (paired with ${agent}) missing`);
  }
}

// --- invariants ---
const cargo = readTextOrFail(path.join(ROOT, "Cargo.toml"), "Cargo.toml");
report.invariants.forbidden_crates = [];
for (const crate of ["tokio", "async-std", "reqwest", "hyper", "actix", "axum"]) {
  if (new RegExp(`^${crate}(\\.[\\w-]+)?\\s*=`, "m").test(cargo) || cargo.includes(`"${crate}"`)) {
    report.invariants.forbidden_crates.push(crate);
    fail(`Cargo.toml pulls forbidden crate ${crate}`);
  }
}
const css = readTextOrFail(path.join(ROOT, "src/css.rs"), "src/css.rs");
const cssHits = [];
if (/^\s*--[a-zA-Z]/.test(css) || /[^A-Za-z]--[a-zA-Z-]+:/.test(css)) {
  cssHits.push("css-custom-properties");
}
if (/\bgap\s*:/.test(css)) cssHits.push("gap");
if (/\binset\s*:/.test(css)) cssHits.push("inset");
report.invariants.css_forbidden = cssHits;
for (const h of cssHits) fail(`src/css.rs still emits GTK-illegal '${h}'`);

report.invariants.laptop_rule = readTextOrFail(
  path.join(ROOT, "CONTRIBUTING.md"),
  "CONTRIBUTING.md",
).includes("geen Rust-toolchain");

// disjoint owns: parse graph.yaml write lists
const owns = {};
for (const m of graphText.matchAll(
  /- id: ([a-z0-9-]+)[\s\S]*?writes: \[([^\]]*)\]/g,
)) {
  owns[m[1]] = m[2]
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}
const fileToNodes = new Map();
for (const [node, files] of Object.entries(owns)) {
  for (const f of files) {
    if (f === "[]" || f.startsWith("diff") || f.startsWith("inline") || f === "") {
      continue;
    }
    const list = fileToNodes.get(f) || [];
    list.push(node);
    fileToNodes.set(f, list);
  }
}
report.invariants.owns_overlap = [];
for (const [f, nodes] of fileToNodes) {
  if (nodes.length > 1) {
    report.invariants.owns_overlap.push({ file: f, nodes });
    fail(`owns overlap on ${f}: ${nodes.join(", ")}`);
  }
}

// --- routing ---
const routingPath = path.join(CURSOR, "evals", "routing.json");
if (!fs.existsSync(routingPath)) {
  fail("missing .cursor/evals/routing.json");
} else {
  const routing = loadJson(routingPath);
  const corpus = [
    ...report.skills.map((s) => ({ kind: "skill", name: s.name, index: s.index })),
    ...report.agents.map((a) => ({ kind: "agent", name: a.name, index: a.index })),
  ];
  for (const c of routing.cases || []) {
    const ranked = corpus
      .map((item) => ({ ...item, score: overlap(c.prompt, item.index || "") }))
      .sort((a, b) => b.score - a.score);
    const topSkills = ranked.filter((r) => r.kind === "skill").slice(0, 3);
    const topAgents = ranked.filter((r) => r.kind === "agent").slice(0, 3);
    const expectSkills = c.expect_skills || [];
    const expectAgents = c.expect_agents || [];
    const skillHit =
      expectSkills.length === 0 ||
      expectSkills.some((s) => topSkills.slice(0, 2).some((t) => t.name === s));
    const agentHit =
      expectAgents.length === 0 ||
      expectAgents.some((s) => topAgents.slice(0, 2).some((t) => t.name === s));
    const forbidOk = (c.forbidden_skills || []).every(
      (s) => topSkills[0]?.name !== s,
    );
    const passed = skillHit && agentHit && forbidOk;
    report.routing.cases.push({
      id: c.id,
      prompt: c.prompt,
      passed,
      top_skills: topSkills.map((t) => `${t.name}:${t.score.toFixed(1)}`),
      top_agents: topAgents.map((t) => `${t.name}:${t.score.toFixed(1)}`),
    });
    report.routing.total += 1;
    if (passed) report.routing.passed += 1;
    else
      report.warnings.push(
        `routing ${c.id} missed (skills ${topSkills.map((t) => t.name).join(", ")})`,
      );
  }
  report.routing.accuracy =
    report.routing.total === 0
      ? 0
      : report.routing.passed / report.routing.total;
}

// --- per-skill trigger files ---
for (const rec of report.skills) {
  const trigPath = path.join(CURSOR, "skills", rec.name, "evals", "triggers.json");
  if (!fs.existsSync(trigPath)) continue;
  let trig;
  try {
    trig = loadJson(trigPath);
  } catch (err) {
    fail(`skill ${rec.name}: triggers.json parse ${err.message}`);
    continue;
  }
  rec.trigger_results = { hit: 0, miss: 0, avoid: 0, leak: 0 };
  const desc = rec.description || "";
  for (const t of trig.should_trigger || []) {
    const terms = t.must_match_description || [];
    const ok = terms.every((term) => desc.toLowerCase().includes(String(term).toLowerCase()));
    if (ok) rec.trigger_results.hit += 1;
    else {
      rec.trigger_results.miss += 1;
      fail(
        `skill ${rec.name}: trigger ${t.id} — description missing [${terms.filter((x) => !desc.toLowerCase().includes(String(x).toLowerCase())).join(", ")}]`,
      );
    }
  }
  for (const t of trig.should_not_trigger || []) {
    const terms = t.must_not_all_match || [];
    const all = terms.length > 0 && terms.every((term) => desc.toLowerCase().includes(String(term).toLowerCase()));
    if (all) {
      rec.trigger_results.leak += 1;
      fail(`skill ${rec.name}: should_not ${t.id} — description over-matches`);
    } else rec.trigger_results.avoid += 1;
  }
}

// --- scores ---
const structFails = report.blocking.filter((b) => !b.startsWith("routing ")).length;
report.score.structure = structFails === 0 ? 100 : Math.max(0, 100 - structFails * 4);
report.score.routing = Math.round(report.routing.accuracy * 100);
const q =
  report.skills.reduce((s, r) => s + (r.quality_score || 0), 0) /
  Math.max(1, report.skills.length);
report.score.quality = Math.round(q);
report.score.overall = Math.round(
  report.score.structure * 0.4 + report.score.routing * 0.35 + report.score.quality * 0.25,
);

const routingFail = report.routing.accuracy < minRouting;
const failed = report.blocking.length > 0 || routingFail;
if (routingFail) {
  report.blocking.push(
    `routing accuracy ${(report.routing.accuracy * 100).toFixed(1)}% < min ${minRouting * 100}%`,
  );
}

const outDir = path.join(CURSOR, "evals");
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(path.join(outDir, "last-report.json"), JSON.stringify(report, null, 2));

if (jsonOnly) {
  console.log(JSON.stringify(report, null, 2));
} else {
  console.log(`ChefBar agent benchmark  overall ${report.score.overall}/100`);
  console.log(
    `  structure ${report.score.structure}  routing ${report.score.routing}% (${report.routing.passed}/${report.routing.total})  quality ${report.score.quality}`,
  );
  console.log(
    `  skills ${report.skills.length}  agents ${report.agents.length}  commands ${report.commands.length}  rules ${report.rules.length}  evals ${report.evals.skills_with_evals}`,
  );
  if (report.blocking.length) {
    console.log("\nBLOCKING");
    for (const b of report.blocking) console.log(`  - ${b}`);
  }
  if (report.warnings.length) {
    console.log("\nWARNINGS");
    for (const b of report.warnings) console.log(`  - ${b}`);
  }
  if (!failed) console.log("\nPASS");
  else console.log("\nFAIL");
}

process.exit(failed ? 1 : 0);
