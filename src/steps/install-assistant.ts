import { execSync } from 'child_process';
import { ASSISTANTS, type SupportedAssistant } from '../assistants.js';

export async function installAssistant(assistant: SupportedAssistant): Promise<void> {
  execSync(ASSISTANTS[assistant].installCommand, { stdio: 'inherit' });
}

export async function updateAssistant(assistant: SupportedAssistant): Promise<void> {
  const command = ASSISTANTS[assistant].updateCommand;
  if (!command) {
    return;
  }

  execSync(command, { stdio: 'inherit' });
}
