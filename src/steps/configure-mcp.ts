import fs from 'fs';
import path from 'path';

interface McpSettings {
  mcpServers: {
    [key: string]: {
      command: string;
      args: string[];
    };
  };
}

export async function configureMcp(projectDir: string): Promise<void> {
  const claudeDir = path.join(projectDir, '.claude');
  const settingsPath = path.join(claudeDir, 'settings.json');

  // .claude 디렉토리 생성
  if (!fs.existsSync(claudeDir)) {
    fs.mkdirSync(claudeDir, { recursive: true });
  }

  // 기존 설정 읽기 (있는 경우)
  let existingSettings: McpSettings = { mcpServers: {} };
  if (fs.existsSync(settingsPath)) {
    try {
      const content = fs.readFileSync(settingsPath, 'utf-8');
      existingSettings = JSON.parse(content);
    } catch {
      // 파싱 실패 시 기본값 사용
    }
  }

  // portone MCP 서버 설정 추가/업데이트
  const settings: McpSettings = {
    ...existingSettings,
    mcpServers: {
      ...existingSettings.mcpServers,
      portone: {
        command: 'npx',
        args: ['-y', '@portone/mcp-server@latest']
      }
    }
  };

  fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2));
}
