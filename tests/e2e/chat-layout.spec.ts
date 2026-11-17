import { test, expect, type Page } from '@playwright/test';

async function fixture(page: Page, answer?: string, kokoroReady = true) {
  const response =
    answer ??
    'Let’s make room for what matters. Start by choosing one important task, then set aside a focused block of time. Leave a little space for breaks and unexpected work. ' +
      'A longer answer should wrap naturally inside the conversation, rather than stretching the page. '.repeat(
        6,
      );
  await page.route('**/src/infrastructure/tauri/assistant-api.ts*', (route) =>
    route.fulfill({
      contentType: 'application/javascript',
      body: `
      export const desktop = true;
      const conversation = {id:'fixture',title:'Layout fixture',updated_at:1};
      const messages = [
        {id:'user',conversation_id:'fixture',role:'user',content:'Help me plan my day.',status:'complete',created_at:1},
        {id:'assistant',conversation_id:'fixture',role:'assistant',content:${JSON.stringify(response)},status:'complete',created_at:1}
      ];
      export const api = {
        bootstrap:async()=>({conversations:[conversation],activity:[],platform:{name:'Windows',automation_available:false},permissions:[],endpoint:'http://127.0.0.1:1234/v1'}),
        models:async()=>[{id:'qwen-fixture'}], messages:async()=>messages, activity:async()=>[],
        search:async(query)=>(conversation.title+' '+messages.map(message=>message.content).join(' ')).toLowerCase().includes(query.toLowerCase()) ? [conversation] : [],
        create:async()=>conversation, delete:async()=>{}, cancel:async()=>{},
        send:async(id,content,model,onEvent)=>{globalThis.__sentVoiceText=content;onEvent({kind:'started',message:{id:'pending',conversation_id:id,role:'assistant',content:'',status:'streaming',created_at:1}});onEvent({kind:'thinking'});globalThis.__emitChatDelta=(text)=>{onEvent({kind:'delta',text});if(globalThis.__voiceEnabled)globalThis.__startNativeSpeech?.()};return await new Promise(resolve=>{globalThis.__completeChat=()=>resolve({message:{id:'pending',conversation_id:id,role:'assistant',content:'Two plus two is four.',status:'complete',created_at:1},error:null})});}
      };
    `,
    }),
  );
  await page.route('**/src/infrastructure/tauri/voice-api.ts*', (route) =>
    route.fulfill({
      contentType: 'application/javascript',
      body: `
      let stage='idle', speaking=false;
      globalThis.__startNativeSpeech=()=>{speaking=true;globalThis.__spokenReplyCount=(globalThis.__spokenReplyCount??0)+1;globalThis.__finishVoiceReply=()=>{speaking=false}};
      export const voiceApi = {
        status:async()=>({stage,model:{ready:true,bytes:147951465},downloaded_bytes:0,recording:{active:stage==='recording',seconds:1},speaking}),
        start:async()=>{stage='recording'},
        finish:async()=>{stage='idle';return 'Voice transcript for review.'},
        cancel:async()=>{stage='idle'},download:async()=>{},
        speak:async()=>{speaking=true;globalThis.__spokenReplyCount=(globalThis.__spokenReplyCount??0)+1;globalThis.__finishVoiceReply=()=>{speaking=false}},stop:async()=>{speaking=false}
      };
    `,
    }),
  );
  await page.route('**/src/infrastructure/tauri/speech-api.ts*', (route) =>
    route.fulfill({
      contentType: 'application/javascript',
      body: `
      let preferences = {voice:'bm_george',speed:0.95,volume:1}, phase='idle', rejectDownload;
      export const speechApi = {
        status:async()=>({preferences,phase,model:{ready:${kokoroReady},bytes:148782633},downloaded_bytes:phase==='downloading'?50000000:0,error:null}),
        save:async(value)=>{preferences={...value}},
        preview:async(value)=>{globalThis.__speechPreview={...value};phase='playing'},
        download:async()=>{phase='downloading';await new Promise((_,reject)=>{rejectDownload=reject})},
        stop:async()=>{phase='idle';if(rejectDownload){rejectDownload('Download cancelled');rejectDownload=null}}
      };
    `,
    }),
  );
  await page.route('**/src/infrastructure/tauri/voice-mode-api.ts*', (route) =>
    route.fulfill({
      contentType: 'application/javascript',
      body: `let enabled=false,phase='off',emit;
      export const voiceModeApi={
        status:async()=>({enabled,phase,model:{ready:true,bytes:6682588},downloaded_bytes:0,error:null}),download:async()=>{},
        start:async(callback)=>{enabled=true;globalThis.__voiceEnabled=true;phase='waiting';emit=callback;globalThis.__emitVoiceMode=callback;callback({kind:'phase',phase});return 'session'},
        heartbeat:async()=>{},resume:async()=>{globalThis.__voiceResumeCount=(globalThis.__voiceResumeCount??0)+1;phase='listening';emit({kind:'phase',phase})},
        stop:async()=>{enabled=false;globalThis.__voiceEnabled=false;phase='off';globalThis.__finishVoiceReply?.()}
      };`,
    }),
  );
  await page.goto('/');
  await page.getByTitle('Layout fixture').click();
  await expect(page.locator('[data-slot="message"]').first()).toBeVisible();
}

test('shadcn Chat aligns user bubbles right and assistant replies left', async ({ page }) => {
  await fixture(page);
  const thread = await page.locator('.messages').boundingBox();
  expect(thread).not.toBeNull();
  for (const card of await page.locator('[data-slot="message"]').all()) {
    const box = (await card.boundingBox())!;
    const avatar = (await card.locator('.message-avatar').boundingBox())!;
    const body = (await card.locator('.message-body').boundingBox())!;
    expect(box.width).toBeGreaterThan(thread!.width * 0.8);
    if ((await card.getAttribute('data-align')) === 'end') {
      expect(body.x + body.width).toBeLessThanOrEqual(avatar.x + 1);
    } else {
      expect(body.x).toBeGreaterThanOrEqual(avatar.x + avatar.width);
    }
    expect(Math.abs(body.y - avatar.y)).toBeLessThan(2);
  }
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
});

test('voice mode turns a spoken request into a spoken reply and resumes listening', async ({
  page,
}) => {
  await fixture(page);
  await page.getByRole('button', { name: 'Voice mode', exact: true }).click();
  await expect(page.getByRole('region', { name: 'Voice conversation' })).toBeVisible();
  await expect(page.getByText('Say “Hey Jarvis”', { exact: true })).toBeVisible();
  const meter = page.getByRole('meter', { name: 'Microphone input level' });
  await expect(meter).toHaveAttribute('aria-valuenow', '0');
  await page.evaluate(() => {
    (window as Window & { __emitVoiceMode: (event: unknown) => void }).__emitVoiceMode({
      kind: 'input_level',
      level: 0.7,
    });
  });
  await expect(meter).toHaveAttribute('aria-valuenow', '70');
  await expect(page.getByText('Audio detected · microphone is receiving sound')).toBeVisible();
  await page.evaluate(() => {
    (window as Window & { __emitVoiceMode: (event: unknown) => void }).__emitVoiceMode({
      kind: 'phase',
      phase: 'listening',
    });
  });
  await expect(page.getByText('I’m listening', { exact: true })).toBeVisible();
  await page.evaluate(() => {
    (window as Window & { __emitVoiceMode: (event: unknown) => void }).__emitVoiceMode({
      kind: 'transcript',
      text: 'What is two plus two?',
    });
  });
  await expect(page.getByText('Thinking it through…', { exact: true })).toBeVisible();
  expect(
    await page.evaluate(() => (window as Window & { __sentVoiceText: string }).__sentVoiceText),
  ).toBe('What is two plus two?');
  await page.evaluate(() =>
    (window as Window & { __emitChatDelta: (text: string) => void }).__emitChatDelta(
      'Two plus two is four. ',
    ),
  );
  // Native speech starts while send_message is still waiting for the rest of the model stream.
  await expect(page.getByText('JARVIS is speaking', { exact: true })).toBeVisible();
  expect(
    await page.evaluate(
      () => (window as Window & { __voiceResumeCount?: number }).__voiceResumeCount ?? 0,
    ),
  ).toBe(0);
  await page.evaluate(() => (window as Window & { __completeChat: () => void }).__completeChat());
  await expect(page.getByText('JARVIS is speaking', { exact: true })).toBeVisible();
  await expect(page.getByText('Two plus two is four.', { exact: true })).toBeVisible();
  expect(
    await page.evaluate(
      () => (window as Window & { __spokenReplyCount: number }).__spokenReplyCount,
    ),
  ).toBe(1);
  await page.evaluate(() =>
    (window as Window & { __finishVoiceReply: () => void }).__finishVoiceReply(),
  );
  await expect(page.getByText('I’m listening', { exact: true })).toBeVisible();
  await page.screenshot({
    path: test.info().outputPath('voice-conversation.png'),
    animations: 'disabled',
  });
  await page.getByRole('button', { name: 'Stop voice mode', exact: true }).click();
  await expect(page.getByLabel('Message JARVIS')).toBeVisible();
});

test('British voice preview is independent of saved reply preferences', async ({ page }) => {
  await fixture(page);
  await page.getByRole('button', { name: 'Voice settings', exact: true }).click();
  await page.getByRole('combobox', { name: 'Voice', exact: true }).click();
  await page.getByRole('option', { name: 'Fable · British male' }).click();
  const speed = page.getByRole('slider', { name: 'Speaking speed' });
  await speed.focus();
  await page.keyboard.press('ArrowRight');
  await expect(speed).toHaveAttribute('aria-valuenow', '1');
  const volume = page.getByRole('slider', { name: 'Output volume' });
  await volume.focus();
  await page.keyboard.press('ArrowRight');
  await expect(volume).toHaveAttribute('aria-valuenow', '1.05');
  await page.getByRole('button', { name: 'Preview voice', exact: true }).click();
  await expect(page.getByText('Playing voice', { exact: true })).toBeVisible();
  expect(
    await page.evaluate(() => (window as Window & { __speechPreview: unknown }).__speechPreview),
  ).toEqual({ voice: 'bm_fable', speed: 1, volume: 1.05 });
  await page.getByRole('button', { name: 'Stop voice', exact: true }).click();
  await page.getByRole('button', { name: 'Save voice', exact: true }).click();
  await expect(page.getByText('Voice saved for future replies.')).toBeVisible();
  await page.getByRole('button', { name: 'Close', exact: true }).click();
  await page.getByRole('button', { name: 'Voice settings', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Voice', exact: true })).toContainText('Fable');
  await expect(page.getByRole('slider', { name: 'Output volume' })).toHaveAttribute(
    'aria-valuenow',
    '1.05',
  );
  await expect(page.getByRole('slider', { name: 'Speaking speed' })).toHaveAttribute(
    'aria-valuenow',
    '1',
  );
  await page.screenshot({
    path: test.info().outputPath('kokoro-settings.png'),
    animations: 'disabled',
  });
});

test('voice pack setup requires an explicit download and remains cancellable', async ({ page }) => {
  await fixture(page, undefined, false);
  await page.getByRole('button', { name: 'Voice settings', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Preview voice', exact: true })).toBeDisabled();
  const download = page.getByRole('button', { name: 'Download Kokoro · 149 MB', exact: true });
  await download.click();
  await expect(page.getByRole('progressbar', { name: 'Kokoro download progress' })).toBeVisible();
  await page.getByRole('button', { name: 'Cancel download', exact: true }).click();
  await expect(download).toBeEnabled();
  await expect(page.getByRole('progressbar', { name: 'Kokoro download progress' })).toHaveCount(0);
  await expect(page.locator('.speech-settings [role="alert"]')).toHaveCount(0);
});

test('dictation appends to the draft for review without sending', async ({ page }) => {
  await fixture(page);
  const draft = page.getByLabel('Message JARVIS');
  await draft.fill('Existing draft.');
  await page.getByRole('button', { name: 'Record voice message', exact: true }).click();
  await expect(page.getByText('Listening · 1 / 30s')).toBeVisible();
  await expect(draft).toBeDisabled();
  await page.getByRole('button', { name: 'Finish', exact: true }).click();
  await expect(draft).toHaveValue('Existing draft.\nVoice transcript for review.');
  await expect(draft).toBeEnabled();
  await expect(page.locator('.generation-status')).toHaveCount(0);
  await expect(page.locator('[data-slot="message"]')).toHaveCount(2);
  await page.screenshot({ path: test.info().outputPath('voice-draft.png') });
});

test('reply playback starts only on request and can be stopped', async ({ page }) => {
  await fixture(page);
  await expect(page.getByText('Reading reply aloud', { exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Read reply aloud' }).click();
  await expect(page.getByText('Reading reply aloud', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Stop speaking' }).click();
  await expect(page.getByText('Reading reply aloud', { exact: true })).toHaveCount(0);
});

test('renders both tuned orb sizes for thinking and pauses them under reduced motion', async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await fixture(page);
  await page.getByLabel('Message JARVIS').fill('Think through my priorities.');
  await page.getByRole('button', { name: 'Send message' }).click();
  await expect(page.locator('.generation-status')).toContainText('JARVIS is thinking');
  await expect(page.locator('.generation-status')).toHaveCSS('opacity', '1');
  const avatar = page.locator('.message-avatar canvas[data-orb-state="solving"]');
  const inline = page.locator('.generation-status canvas[data-orb-state="solving"]');
  await expect(avatar).toHaveAttribute('data-orb-paused', 'true');
  await expect(inline).toHaveAttribute('data-orb-paused', 'true');
  expect((await avatar.boundingBox())!.width).toBe(64);
  expect((await inline.boundingBox())!.width).toBe(20);
  expect(
    await avatar.evaluate((element: HTMLCanvasElement) => {
      const pixels = element
        .getContext('2d')!
        .getImageData(0, 0, element.width, element.height).data;
      return pixels.some((value, index) => index % 4 === 3 && value > 0);
    }),
  ).toBe(true);
  await page.screenshot({ path: test.info().outputPath('chat-thinking.png') });
});

test('conversation fits the minimum desktop width', async ({ page }) => {
  await page.setViewportSize({ width: 760, height: 700 });
  await fixture(page);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  const body = await page.locator('.message-body').first().boundingBox();
  expect(body!.width).toBeGreaterThan(200);
});

test('shadcn scrolling preserves older reading position and can return to the live reply', async ({
  page,
}) => {
  await fixture(page);
  await page.getByLabel('Message JARVIS').fill('Please continue.');
  await page.getByRole('button', { name: 'Send message' }).click();
  await expect(page.locator('.generation-status')).toContainText('JARVIS is thinking');
  const emit = async (text: string) =>
    page.evaluate((text) => {
      (window as Window & { __emitChatDelta: (text: string) => void }).__emitChatDelta(text);
    }, text);
  await emit('A streaming paragraph with useful details.\n\n'.repeat(80));
  const reply = page.locator('[data-slot="message"]').last();
  await expect(reply).toContainText('A streaming paragraph');
  const viewport = page.locator('[data-slot="message-scroller-viewport"]');
  await viewport.hover();
  await page.mouse.wheel(0, -10000);
  await expect.poll(() => viewport.evaluate((element) => element.scrollTop)).toBeLessThan(5);
  await emit('\n\nNew detail while you are reading earlier messages.');
  await expect(reply).toContainText('New detail while you are reading');
  await page.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      ),
  );
  expect(await viewport.evaluate((element) => element.scrollTop)).toBeLessThan(5);
  const latest = page.getByRole('button', { name: 'Latest message' });
  await expect(latest).toHaveAttribute('data-active', 'true');
  await latest.click();
  await expect
    .poll(() =>
      viewport.evaluate(
        (element) => element.scrollHeight - element.scrollTop - element.clientHeight,
      ),
    )
    .toBeLessThan(5);
});

test('search finds a conversation by saved message text and clearing restores history', async ({
  page,
}) => {
  await fixture(page);
  await page.getByLabel('Search conversations').fill('longer answer');
  await expect(page.getByText('Searching locally…')).toHaveCount(0);
  await expect(page.getByTitle('Layout fixture')).toBeVisible();
  await page.getByLabel('Search conversations').fill('no matching phrase');
  await expect(page.getByText('No matching conversations.')).toBeVisible();
  await expect(page.getByTitle('Layout fixture')).toHaveCount(0);
  await page.getByLabel('Clear conversation search').click();
  await expect(page.getByTitle('Layout fixture')).toBeVisible();
});

test('formats answers without fetching images or allowing model-generated navigation', async ({
  page,
}) => {
  const remoteRequests: string[] = [];
  page.on('request', (request) => {
    if (request.url().startsWith('https://example.invalid')) remoteRequests.push(request.url());
  });
  await page.setViewportSize({ width: 760, height: 700 });
  await fixture(
    page,
    '# Your plan\n\n- **Focus** first\n- Take a break\n\n```ts\n' +
      'const example = "' +
      'x'.repeat(300) +
      '";\n```\n\n| Task | Time |\n| --- | --- |\n| Build | Morning |\n\n[Reference](https://example.invalid/docs)\n\n![Private image](https://example.invalid/pixel.png)\n\n<script>alert(1)</script>',
  );
  await expect(page.getByRole('heading', { name: 'Your plan', level: 2 })).toBeVisible();
  await expect(page.locator('pre code')).toContainText('const example');
  await expect(page.getByRole('table')).toBeAttached();
  await expect(
    page.locator('.message-content a,.message-content img,.message-content script'),
  ).toHaveCount(0);
  expect(remoteRequests).toHaveLength(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  await page.screenshot({ path: test.info().outputPath('formatted-chat.png') });
});
