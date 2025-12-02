import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

export async function checkClaudeInstalled(): Promise<boolean> {
  try {
    const { stdout } = await execAsync('claude --version');
    console.log(stdout);
    return stdout.includes('Claude Code');
  } catch (error) {
    console.error(error);
    return false;
  }
}
