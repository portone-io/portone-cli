import { execSync } from 'child_process';

export async function checkClaudeInstalled(): Promise<boolean> {
  try {
    execSync('claude --version', { stdio: 'pipe' });
    return true;
  } catch {
    return false;
  }
}
