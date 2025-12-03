import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

export async function configurePlugin(projectDir: string): Promise<void> {
  await execAsync('claude plugin marketplace remove portone', { cwd: projectDir });
  await execAsync('claude plugin marketplace add portone-io/portone-cli', { cwd: projectDir });
  await execAsync('claude plugin install portone-integration', { cwd: projectDir });
}
