import { confirm } from '@inquirer/prompts';
import ora from 'ora';
import chalk from 'chalk';
import { checkClaudeInstalled } from '../steps/check-claude.js';
import { installClaude } from '../steps/install-claude.js';
import { configurePlugin } from '../steps/configure-plugin.js';
import { showIntegrationGuide } from '../steps/run-integration.js';
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

    const shouldInstall = await confirm({
      message: 'Claude Code를 설치하시겠습니까?',
      default: true
    });

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

  // Step 2: 플러그인 설정
  spinner = ora('PortOne 플러그인 설정 중...').start();
  try {
    await configurePlugin(process.cwd());
    spinner.succeed('플러그인 설정 완료');
  } catch (error) {
    spinner.fail('플러그인 설정 실패');
    console.error(chalk.red(error instanceof Error ? error.message : String(error)));
    process.exit(1);
  }

  // Step 4: 안내 출력
  console.log(chalk.green('\n✅ 설정이 완료되었습니다!'));
  showIntegrationGuide();
}
