import { spawn } from 'child_process';
import { getIntegrationPrompt } from '../utils/prompts.js';

export interface IntegrationOptions {
  type: 'payment' | 'identity';
  version: 'v1' | 'v2';
  pg?: string;
  method?: string;
}

interface StreamEvent {
  type: string;
  content?: string;
  message?: {
    content?: string;
  };
}

export async function runIntegration(
  projectDir: string,
  options: IntegrationOptions
): Promise<void> {
  const prompt = getIntegrationPrompt(options);

  return new Promise((resolve, reject) => {
    // Claude Code headless 모드로 실행
    const claude = spawn('claude', ['-p', prompt, '--output-format', 'stream-json'], {
      cwd: projectDir,
      stdio: ['pipe', 'pipe', 'pipe'],
      shell: true
    });

    let buffer = '';

    // 실시간 출력 처리
    claude.stdout.on('data', (data: Buffer) => {
      buffer += data.toString();

      // 줄바꿈으로 구분된 JSON 파싱
      const lines = buffer.split('\n');
      buffer = lines.pop() || ''; // 마지막 불완전한 줄은 버퍼에 유지

      for (const line of lines) {
        if (!line.trim()) continue;

        try {
          const event: StreamEvent = JSON.parse(line);

          // assistant 메시지 출력
          if (event.type === 'assistant' && event.message?.content) {
            process.stdout.write(event.message.content);
          }
          // text 이벤트 출력
          else if (event.type === 'text' && event.content) {
            process.stdout.write(event.content);
          }
        } catch {
          // JSON 파싱 실패 시 무시
        }
      }
    });

    claude.stderr.on('data', (data: Buffer) => {
      // stderr는 디버그 정보일 수 있으므로 조용히 처리
      const message = data.toString();
      if (message.includes('error') || message.includes('Error')) {
        console.error('\n' + message);
      }
    });

    claude.on('error', (error) => {
      reject(new Error(`Claude Code 실행 실패: ${error.message}`));
    });

    claude.on('close', (code) => {
      // 남은 버퍼 처리
      if (buffer.trim()) {
        try {
          const event: StreamEvent = JSON.parse(buffer);
          if (event.type === 'assistant' && event.message?.content) {
            process.stdout.write(event.message.content);
          } else if (event.type === 'text' && event.content) {
            process.stdout.write(event.content);
          }
        } catch {
          // 무시
        }
      }

      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`Claude Code가 종료 코드 ${code}로 종료되었습니다`));
      }
    });
  });
}
