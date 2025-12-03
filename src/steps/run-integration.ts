import chalk from 'chalk';

export function showIntegrationGuide(): void {
  const command = '/portone-integration:start';

  console.log(chalk.cyan('\n📋 다음 단계'));
  console.log(chalk.white('─'.repeat(40)));
  console.log(chalk.white('\n1. Claude Code를 실행하세요:'));
  console.log(chalk.yellow('   $ claude\n'));
  console.log(chalk.white('2. 아래 슬래시 커맨드를 입력하세요:'));
  console.log(chalk.green(`   ${command}\n`));
  console.log(chalk.white('─'.repeat(40)));
}
