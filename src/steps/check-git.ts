import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

export async function isGitClean(): Promise<boolean> {
  try {
    const { stdout } = await execAsync('git status --porcelain');
    return stdout.trim() === '';
  } catch {
    // git 저장소가 아닌 경우 체크 건너뜀
    return true;
  }
}
