import { test, expect, type Page } from '@playwright/test';

async function fixture(page: Page) {
  await page.route('**/src/infrastructure/tauri/assistant-api.ts*', (route) =>
    route.fulfill({
      contentType: 'application/javascript',
      body: `
      export const desktop = true;
      const conversation = {id:'fixture',title:'Layout fixture',updated_at:1};
      const messages = [
        {id:'user',conversation_id:'fixture',role:'user',content:'Help me plan my day.',status:'complete',created_at:1},
        {id:'assistant',conversation_id:'fixture',role:'assistant',content:'Let’s make room for what matters. Start by choosing one important task, then set aside a focused block of time. Leave a little space for breaks and unexpected work. '+ 'A longer answer should wrap naturally inside the conversation, rather than stretching the page. '.repeat(6),status:'complete',created_at:1}
      ];
      export const api = {
        bootstrap:async()=>({conversations:[conversation],activity:[],platform:{name:'Windows',automation_available:false},permissions:[],endpoint:'http://127.0.0.1:1234/v1'}),
        models:async()=>[{id:'qwen-fixture'}], messages:async()=>messages, activity:async()=>[],
        create:async()=>conversation, delete:async()=>{}, cancel:async()=>{},
        send:async(id,content,model,onEvent)=>{onEvent({kind:'started',message:{id:'pending',conversation_id:id,role:'assistant',content:'',status:'streaming',created_at:1}});onEvent({kind:'thinking'});await new Promise(()=>{});}
      };
    `,
    }),
  );
  await page.goto('/');
  await page.getByTitle('Layout fixture').click();
  await expect(page.getByRole('log')).toBeVisible();
}

test('short and long messages fill the thread with avatars beside the text', async ({ page }) => {
  await fixture(page);
  const thread = await page.locator('.messages').boundingBox();
  expect(thread).not.toBeNull();
  for (const card of await page.locator('.message').all()) {
    const box = (await card.boundingBox())!;
    const avatar = (await card.locator('.message-avatar').boundingBox())!;
    const body = (await card.locator('.message-body').boundingBox())!;
    expect(box.width).toBeGreaterThan(thread!.width * 0.8);
    expect(body.x).toBeGreaterThanOrEqual(avatar.x + avatar.width);
    expect(Math.abs(body.y - avatar.y)).toBeLessThan(2);
  }
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
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
