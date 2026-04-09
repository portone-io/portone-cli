import { confirm, select } from '@inquirer/prompts';
import ora from 'ora';
import chalk from 'chalk';
import {
  ASSISTANTS,
  isAssistantSelection,
  resolveAssistantTargets,
  type AssistantSelection
} from '../assistants.js';
import { checkAssistantInstalled } from '../steps/check-assistant.js';
import { installAssistant, updateAssistant } from '../steps/install-assistant.js';
import { configurePlugin } from '../steps/configure-plugin.js';
import { showIntegrationGuide } from '../steps/run-integration.js';
import { isGitClean } from '../steps/check-git.js';

export interface SetupOptions {
  allowDirty?: boolean;
  assistant?: string;
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

  const assistantSelection = await resolveAssistantSelection(options.assistant);
  const assistants = resolveAssistantTargets(assistantSelection);

  for (const assistant of assistants) {
    const definition = ASSISTANTS[assistant];

    let spinner = ora(`${definition.displayName} 설치 확인 중...`).start();
    const installed = await checkAssistantInstalled(assistant);

    if (!installed) {
      spinner.warn(`${definition.displayName}가 설치되어 있지 않습니다`);

      const shouldInstall = await confirm({
        message: `${definition.displayName}를 설치하시겠습니까?`,
        default: true
      });

      if (shouldInstall) {
        spinner = ora(`${definition.displayName} 설치 중...`).start();
        try {
          await installAssistant(assistant);
          spinner.succeed(`${definition.displayName} 설치 완료`);
        } catch {
          spinner.fail(`${definition.displayName} 설치 실패`);
          console.log(chalk.yellow(`\n${definition.displayName} 수동 설치: ${definition.installHint}`));
          process.exit(1);
        }
      } else {
        console.log(chalk.yellow(`\n${definition.displayName} 수동 설치: ${definition.installHint}`));
        process.exit(1);
      }
    } else {
      spinner.succeed(`${definition.displayName} 설치 확인됨`);
    }

    if (definition.updateCommand) {
      spinner = ora(`${definition.displayName} 업데이트 중...`).start();
      try {
        await updateAssistant(assistant);
        spinner.succeed(`${definition.displayName} 업데이트 완료`);
      } catch {
        spinner.warn(`${definition.displayName} 업데이트 실패 (계속 진행합니다)`);
      }
    }

    spinner = ora(`PortOne 플러그인 설정 중... (${definition.displayName})`).start();
    try {
      await configurePlugin(assistant, process.cwd());
      spinner.succeed(`플러그인 설정 완료 (${definition.displayName})`);
    } catch (error) {
      spinner.fail(`플러그인 설정 실패 (${definition.displayName})`);
      console.error(chalk.red(error instanceof Error ? error.message : String(error)));
      process.exit(1);
    }
  }

  console.log(chalk.green('\n✅ 설정이 완료되었습니다!'));
  showIntegrationGuide(assistants);
}

async function resolveAssistantSelection(input?: string): Promise<AssistantSelection> {
  if (input) {
    if (!isAssistantSelection(input)) {
      console.log(chalk.red(`지원하지 않는 assistant입니다: ${input}`));
      process.exit(1);
    }

    return input;
  }

  return select<AssistantSelection>({
    message: '어떤 assistant를 설정하시겠습니까?',
    default: 'both',
    choices: [
      { name: 'Claude Code + Codex', value: 'both' },
      { name: 'Claude Code', value: 'claude' },
      { name: 'Codex', value: 'codex' }
    ]
  });
}
