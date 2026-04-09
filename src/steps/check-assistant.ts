import { exec } from 'child_process';
import { promisify } from 'util';
import { ASSISTANTS, type SupportedAssistant } from '../assistants.js';

const execAsync = promisify(exec);

export async function checkAssistantInstalled(assistant: SupportedAssistant): Promise<boolean> {
  const definition = ASSISTANTS[assistant];

  try {
    const { stdout, stderr } = await execAsync(definition.versionCommand);
    const output = `${stdout}${stderr}`;
    return definition.validateVersionOutput(output);
  } catch {
    return false;
  }
}
