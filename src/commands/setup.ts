import inquirer from 'inquirer';
import ora from 'ora';
import chalk from 'chalk';
import { checkClaudeInstalled } from '../steps/check-claude.js';
import { installClaude } from '../steps/install-claude.js';
import { configureMcp } from '../steps/configure-mcp.js';
import { runIntegration } from '../steps/run-integration.js';
import { isGitClean } from '../steps/check-git.js';

export interface SetupOptions {
  allowDirty?: boolean;
}

export async function setup(options: SetupOptions = {}) {
  console.log(chalk.bold('\n🚀 PortOne 연동 설정을 시작합니다\n'));

  // Step 0: Git 상태 확인
  if (!options.allowDirty) {
    const gitSpinner = ora('Git 상태 확인 중...').start();
    const clean = await isGitClean();
    if (!clean) {
      gitSpinner.fail('Git에 커밋되지 않은 변경사항이 있습니다');
      console.log(chalk.yellow('\n변경사항을 커밋하거나 --allow-dirty 플래그를 사용하세요'));
      process.exit(1);
    }
    gitSpinner.succeed('Git 상태 확인됨');
  }

  // Step 1: Claude Code 설치 확인
  let spinner = ora('Claude Code 설치 확인 중...').start();
  const isClaudeInstalled = await checkClaudeInstalled();

  if (!isClaudeInstalled) {
    spinner.warn('Claude Code가 설치되어 있지 않습니다');

    const { shouldInstall } = await inquirer.prompt([{
      type: 'confirm',
      name: 'shouldInstall',
      message: 'Claude Code를 설치하시겠습니까?',
      default: true
    }]);

    if (shouldInstall) {
      spinner = ora('Claude Code 설치 중...').start();
      try {
        await installClaude();
        spinner.succeed('Claude Code 설치 완료');
      } catch (error) {
        spinner.fail('Claude Code 설치 실패');
        console.log(chalk.yellow('\nClaude Code 수동 설치: npm install -g @anthropic-ai/claude-code'));
        process.exit(1);
      }
    } else {
      console.log(chalk.yellow('\nClaude Code 수동 설치: npm install -g @anthropic-ai/claude-code'));
      process.exit(1);
    }
  } else {
    spinner.succeed('Claude Code 설치 확인됨');
  }

  // Step 2: MCP 서버 설정
  spinner = ora('PortOne MCP 서버 설정 중...').start();
  try {
    await configureMcp(process.cwd());
    spinner.succeed('MCP 서버 설정 완료 (.claude/settings.json)');
  } catch (error) {
    spinner.fail('MCP 서버 설정 실패');
    console.error(chalk.red(error instanceof Error ? error.message : String(error)));
    process.exit(1);
  }

  // Step 3: 연동 유형 선택
  const { integrationType } = await inquirer.prompt([{
    type: 'list',
    name: 'integrationType',
    message: '연동 유형을 선택하세요:',
    choices: [
      { name: '💳 결제 연동', value: 'payment' },
      { name: '🔐 본인인증 연동', value: 'identity' }
    ]
  }]);

  const { version } = await inquirer.prompt([{
    type: 'list',
    name: 'version',
    message: '포트원 버전을 선택하세요:',
    choices: [
      { name: 'V2 (권장)', value: 'v2' },
      { name: 'V1 (레거시)', value: 'v1' }
    ]
  }]);

  // Step 4: Claude Code로 연동 실행
  console.log(chalk.cyan('\n✨ Claude Code로 연동을 시작합니다...\n'));

  try {
    await runIntegration(process.cwd(), {
      type: integrationType,
      version
    });
    console.log(chalk.green('\n✅ 연동이 완료되었습니다!\n'));
  } catch (error) {
    console.error(chalk.red('\n❌ 연동 중 오류가 발생했습니다:'));
    console.error(chalk.red(error instanceof Error ? error.message : String(error)));
    process.exit(1);
  }
}
