import { promisify } from 'node:util';
import childProcess from 'node:child_process';

const exec = promisify(childProcess.exec);

export async function checkClaudeInstalled(): Promise<boolean> {
  try {
    const { stdout } = await exec('claude --version');
    console.log(stdout);
    return stdout.includes('Claude Code');
  } catch (error) {
    console.error(error);
    return false;
  }
}
