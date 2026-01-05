import { execSync } from 'child_process';

export async function installClaude(): Promise<void> {
  // npm을 통한 Claude Code 설치
  execSync('npm install -g @anthropic-ai/claude-code', { stdio: 'inherit' });
}

export async function updateClaude(): Promise<void> {
  // Claude Code 업데이트
  execSync('claude update', { stdio: 'inherit' });
}
