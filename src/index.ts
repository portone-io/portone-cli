#!/usr/bin/env node
import { Command } from 'commander';
import { setup } from './commands/setup.js';

const program = new Command();

program
  .name('portone')
  .description('PortOne 결제/본인인증 연동을 위한 CLI 도구')
  .version('0.1.0');

program
  .command('setup')
  .description('Claude Code와 PortOne MCP 서버를 설정하고 연동을 시작합니다')
  .option('--allow-dirty', 'git이 clean 상태가 아니어도 실행을 허용합니다')
  .action(setup);

program.parse();
