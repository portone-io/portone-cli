export type SupportedAssistant = 'claude' | 'codex';
export type AssistantSelection = SupportedAssistant | 'both';

export interface AssistantDefinition {
  id: SupportedAssistant;
  displayName: string;
  cliName: string;
  versionCommand: string;
  installCommand: string;
  installHint: string;
  validateVersionOutput: (output: string) => boolean;
  updateCommand?: string;
}

export const ASSISTANTS: Record<SupportedAssistant, AssistantDefinition> = {
  claude: {
    id: 'claude',
    displayName: 'Claude Code',
    cliName: 'claude',
    versionCommand: 'claude --version',
    installCommand: 'npm install -g @anthropic-ai/claude-code',
    installHint: 'npm install -g @anthropic-ai/claude-code',
    validateVersionOutput: (output) => output.includes('Claude Code'),
    updateCommand: 'claude update'
  },
  codex: {
    id: 'codex',
    displayName: 'Codex',
    cliName: 'codex',
    versionCommand: 'codex --version',
    installCommand: 'npm install -g @openai/codex',
    installHint: 'npm install -g @openai/codex',
    validateVersionOutput: (output) => /codex/i.test(output)
  }
};

export function resolveAssistantTargets(selection: AssistantSelection): SupportedAssistant[] {
  if (selection === 'both') {
    return ['claude', 'codex'];
  }

  return [selection];
}

export function isAssistantSelection(value: string): value is AssistantSelection {
  return value === 'claude' || value === 'codex' || value === 'both';
}
