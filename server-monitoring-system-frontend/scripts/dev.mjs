import { spawn } from 'child_process';
import fs from 'fs';
import path from 'path';

let port = 3000;

try {
    const envPath = path.join(process.cwd(), '.env');
    if (fs.existsSync(envPath)) {
        const envContent = fs.readFileSync(envPath, 'utf8');
        const match = envContent.match(/^PORT\s*=\s*(\d+)/m);
        if (match && match[1]) {
            port = parseInt(match[1], 10);
        }
    }
} catch (e) {
    console.warn('Failed to parse .env for PORT, using default 3000:', e instanceof Error ? e.message : String(e));
}

console.log(`Starting Next.js dev server on port: ${port}`);
const child = spawn('npx', ['next', 'dev', '-p', port.toString()], {
    stdio: 'inherit',
    shell: true,
});

child.on('close', (code) => {
    process.exit(code ?? 0);
});
