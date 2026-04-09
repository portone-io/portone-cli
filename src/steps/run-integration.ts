import chalk from 'chalk';
import { type SupportedAssistant } from '../assistants.js';

export function showIntegrationGuide(assistants: SupportedAssistant[]): void {
  console.log(chalk.cyan('\n📋 다음 단계'));
  console.log(chalk.white('─'.repeat(40)));

  if (assistants.includes('claude')) {
    console.log(chalk.white('\n[Claude Code]'));
    console.log(chalk.white('1. Claude Code를 실행하세요:'));
    console.log(chalk.yellow('   $ claude\n'));
    console.log(chalk.white('2. 아래 슬래시 커맨드를 입력하세요:'));
    console.log(chalk.green('   /portone-integration:start\n'));
  }

  if (assistants.includes('codex')) {
    console.log(chalk.white('[Codex]'));
    console.log(chalk.white('1. Codex를 실행하세요:'));
    console.log(chalk.yellow('   $ codex\n'));
    console.log(chalk.white('2. `portone-codex` 플러그인이 설치된 상태에서 아래처럼 요청하세요:'));
    console.log(chalk.green('   포트원 V2 일반결제 연동 코드 구현해줘'));
    console.log(chalk.green('   프로젝트의 포트원 연동 코드를 검토해줘\n'));
  }

  console.log(chalk.white('─'.repeat(40)));
}
