import { exec } from 'child_process';
import { promises as fs } from 'fs';
import { dirname, join, resolve } from 'path';
import { fileURLToPath } from 'url';
import { promisify } from 'util';
import { type SupportedAssistant } from '../assistants.js';

const execAsync = promisify(exec);

const CLAUDE_PLUGIN_NAME = 'portone-integration';
const CODEX_PLUGIN_NAME = 'portone-codex';
const REPO_MARKETPLACE_NAME = 'portone';
const REPO_MARKETPLACE_DISPLAY_NAME = 'PortOne Plugins';
const LEGACY_CODEX_PLUGIN_NAMES = ['portone-integration'];

interface CodexMarketplaceEntry {
  name: string;
  source: {
    source: 'local';
    path: string;
  };
  policy: {
    installation: 'AVAILABLE';
    authentication: 'ON_INSTALL';
  };
  category: 'Productivity';
}

interface CodexMarketplace {
  name: string;
  interface?: {
    displayName?: string;
  };
  plugins: CodexMarketplaceEntry[];
}

export async function configurePlugin(
  assistant: SupportedAssistant,
  projectDir: string
): Promise<void> {
  if (assistant === 'claude') {
    await configureClaudePlugin(projectDir);
    return;
  }

  await configureCodexPlugin(projectDir);
}

async function configureClaudePlugin(projectDir: string): Promise<void> {
  await execAsync('claude plugin marketplace remove portone', { cwd: projectDir }).catch(() => {});
  await execAsync('claude plugin marketplace add portone-io/portone-cli', { cwd: projectDir });
  await execAsync(`claude plugin install ${CLAUDE_PLUGIN_NAME}`, { cwd: projectDir });
}

async function configureCodexPlugin(projectDir: string): Promise<void> {
  const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  const sourcePluginDir = join(packageRoot, 'plugins', CODEX_PLUGIN_NAME);
  const targetPluginDir = join(projectDir, 'plugins', CODEX_PLUGIN_NAME);
  const marketplacePath = join(projectDir, '.agents', 'plugins', 'marketplace.json');

  await ensureCodexPluginSource(sourcePluginDir);
  await fs.mkdir(join(projectDir, 'plugins'), { recursive: true });
  if (sourcePluginDir !== targetPluginDir) {
    await fs.cp(sourcePluginDir, targetPluginDir, { recursive: true, force: true });
  }
  await updateCodexMarketplace(marketplacePath);
}

async function ensureCodexPluginSource(sourcePluginDir: string): Promise<void> {
  const pluginJsonPath = join(sourcePluginDir, '.codex-plugin', 'plugin.json');

  try {
    await fs.access(pluginJsonPath);
  } catch {
    throw new Error(`Codex plugin assets not found: ${pluginJsonPath}`);
  }
}

async function updateCodexMarketplace(marketplacePath: string): Promise<void> {
  let marketplace = buildDefaultMarketplace();

  try {
    const raw = await fs.readFile(marketplacePath, 'utf8');
    const parsed = JSON.parse(raw) as Partial<CodexMarketplace>;
    marketplace = {
      name: typeof parsed.name === 'string' ? parsed.name : REPO_MARKETPLACE_NAME,
      interface:
        parsed.interface && typeof parsed.interface === 'object'
          ? { displayName: parsed.interface.displayName ?? REPO_MARKETPLACE_DISPLAY_NAME }
          : { displayName: REPO_MARKETPLACE_DISPLAY_NAME },
      plugins: Array.isArray(parsed.plugins) ? parsed.plugins as CodexMarketplaceEntry[] : []
    };
  } catch (error) {
    const isMissing = error instanceof Error && 'code' in error && error.code === 'ENOENT';
    if (!isMissing) {
      throw error;
    }
  }

  const entry = buildMarketplaceEntry();
  const nextPlugins = marketplace.plugins.filter(
    (plugin) => ![CODEX_PLUGIN_NAME, ...LEGACY_CODEX_PLUGIN_NAMES].includes(plugin.name)
  );
  nextPlugins.push(entry);

  marketplace.plugins = nextPlugins;
  marketplace.interface = {
    displayName: marketplace.interface?.displayName ?? REPO_MARKETPLACE_DISPLAY_NAME
  };

  await fs.mkdir(dirname(marketplacePath), { recursive: true });
  await fs.writeFile(marketplacePath, `${JSON.stringify(marketplace, null, 2)}\n`, 'utf8');
}

function buildDefaultMarketplace(): CodexMarketplace {
  return {
    name: REPO_MARKETPLACE_NAME,
    interface: {
      displayName: REPO_MARKETPLACE_DISPLAY_NAME
    },
    plugins: []
  };
}

function buildMarketplaceEntry(): CodexMarketplaceEntry {
  return {
    name: CODEX_PLUGIN_NAME,
    source: {
      source: 'local',
      path: `./plugins/${CODEX_PLUGIN_NAME}`
    },
    policy: {
      installation: 'AVAILABLE',
      authentication: 'ON_INSTALL'
    },
    category: 'Productivity'
  };
}
